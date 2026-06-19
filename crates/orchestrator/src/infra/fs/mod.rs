//! File-tree domain store: `infra/fs/`.
//!
//! # Layout (under `<data-root>/`)
//! ```text
//! workspaces/
//!   <ws-slug>/
//!     workspace.json
//!     projects/
//!       <proj-slug>/
//!         project.json
//!         sessions/
//!           <sess-slug>/
//!             session.json
//!             layout.json
//!         .archive/
//!           <sess-slug>/   ← archived sessions
//!       .archive/
//!         <proj-slug>/     ← archived projects (with their sessions)
//!   .archive/
//!     <ws-slug>/           ← (future) archived workspaces
//! ```
//!
//! # Design decisions implemented
//! - D2: module `infra/fs/`
//! - D3: in-memory `RwLock<TreeState>` with id→path index, built by boot scan
//! - D4: atomic write-temp-rename; archive/delete via `fs::rename` + `fs::remove_dir_all`
//! - D5: slug derivation + collision suffixing; rename = re-slug + subtree move
//! - D6: placement uniqueness enforced under write lock
//! - D9: seed Default workspace + Unfiled project on empty tree
//!
//! # `create_session`
//! `create_session` accepts a resolved `Option<(u32, String)>` (spec_version, spec_json)
//! directly instead of a `template_id`; template->spec resolution lives in the
//! `create_session` coordinator. The `NewSession.template_id` field is ignored here.

pub(crate) use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::RwLock,
};

use serde::{Deserialize, Serialize};

pub(crate) use crate::{
    entities::{
        NewProject, NewSession, NewSurface, NewWorkspace, Project, ProjectId, Session, SessionId,
        SourceKind, Surface, SurfaceId, SurfaceKind, TitleSource, Workspace, WorkspaceId,
    },
    error::{OrchestratorError, Result},
};

mod atomic_io;
mod datetime;
mod index;
mod project;
mod session;
mod slug;
mod surface;
mod workspace;

pub(crate) use atomic_io::{atomic_write, create_dir_secure, persist, read_json, to_json};
pub(crate) use datetime::now_iso8601;
pub(crate) use index::{all_dirs_including_archive, build_index, is_archived, list_live_dirs};
pub(crate) use slug::{slugify, unique_slug};

// ── on-disk JSON shapes ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceFile {
    id: String,
    name: String,
    #[serde(rename = "sortOrder")]
    sort_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectFile {
    id: String,
    name: String,
    #[serde(rename = "sourceKind")]
    source_kind: String,
    #[serde(rename = "rootPath", skip_serializing_if = "Option::is_none")]
    root_path: Option<String>,
    #[serde(rename = "sortOrder")]
    sort_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionFile {
    id: String,
    title: String,
    #[serde(rename = "titleSource")]
    title_source: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "sortOrder")]
    sort_order: u32,
    #[serde(rename = "specVersion", skip_serializing_if = "Option::is_none")]
    spec_version: Option<u32>,
    #[serde(rename = "specJson", skip_serializing_if = "Option::is_none")]
    spec_json: Option<String>,
}

/// A single surface binding stored in `layout.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceBinding {
    pub id: String,
    pub kind: String,
    pub placement: Option<String>,
    /// cwd relative to the project root path.
    pub cwd: Option<String>,
    /// Runtime status (not persisted on disk — optional field for in-memory use).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    /// Whether this binding has been soft-deleted.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LayoutFile {
    /// Opaque panel-tree JSON (stored as raw string, forwarded as-is).
    #[serde(rename = "panelTree", skip_serializing_if = "Option::is_none")]
    panel_tree: Option<serde_json::Value>,
    /// Surface bindings for this session.
    #[serde(default)]
    surfaces: Vec<SurfaceBinding>,
}

// ── in-memory state ───────────────────────────────────────────────────────────

/// In-memory index: maps stable entity id → absolute path to the entity's *directory*.
#[derive(Default)]
struct TreeState {
    /// id → dir path (workspace dirs, project dirs, session dirs).
    index: HashMap<String, PathBuf>,
}

impl TreeState {
    /// Record an id→path mapping, replacing any previous entry.
    fn insert(&mut self, id: &str, path: PathBuf) {
        self.index.insert(id.to_owned(), path);
    }

    /// Remove an id from the index.
    fn remove(&mut self, id: &str) {
        self.index.remove(id);
    }

    /// Look up a directory path by id.
    fn get(&self, id: &str) -> Option<&Path> {
        self.index.get(id).map(PathBuf::as_path)
    }
}

fn source_kind_str(k: SourceKind) -> &'static str {
    match k {
        SourceKind::Blank => "blank",
        SourceKind::LocalDir => "local_dir",
        SourceKind::GitRepo => "git_repo",
    }
}

fn source_kind_from_str(s: &str) -> Result<SourceKind> {
    match s {
        "blank" => Ok(SourceKind::Blank),
        "local_dir" => Ok(SourceKind::LocalDir),
        "git_repo" => Ok(SourceKind::GitRepo),
        other => Err(OrchestratorError::Persistence(format!(
            "unknown source_kind: {other}"
        ))),
    }
}

fn title_source_str(ts: TitleSource) -> &'static str {
    match ts {
        TitleSource::AgentTitle => "agent-title",
        TitleSource::Branch => "branch",
        TitleSource::Both => "both",
        TitleSource::Custom => "custom",
    }
}

fn title_source_from_str(s: &str) -> Result<TitleSource> {
    match s {
        "agent-title" => Ok(TitleSource::AgentTitle),
        "branch" => Ok(TitleSource::Branch),
        "both" => Ok(TitleSource::Both),
        "custom" => Ok(TitleSource::Custom),
        other => Err(OrchestratorError::Persistence(format!(
            "unknown title_source: {other}"
        ))),
    }
}

fn surface_kind_str(k: SurfaceKind) -> &'static str {
    match k {
        SurfaceKind::Terminal => "terminal",
        SurfaceKind::Diff => "diff",
    }
}

fn surface_kind_from_str(s: &str) -> Result<SurfaceKind> {
    match s {
        "terminal" => Ok(SurfaceKind::Terminal),
        "diff" => Ok(SurfaceKind::Diff),
        other => Err(OrchestratorError::UnsupportedSurfaceKind(other.to_owned())),
    }
}

// ── FsBackend ─────────────────────────────────────────────────────────────────

/// File-tree backed domain store.
///
/// Construct via [`FsBackend::open`]. All mutations are serialized through the write lock.
pub struct FsBackend {
    root: PathBuf,
    state: RwLock<TreeState>,
}

impl FsBackend {
    /// Open (or create) a `FsBackend` rooted at `root`.
    ///
    /// On first open (empty tree) the Default workspace and Unfiled project are seeded (D9).
    /// On subsequent opens the in-memory index is rebuilt by scanning the tree.
    pub fn open(root: PathBuf) -> Result<Self> {
        create_dir_secure(&root).map_err(persist)?;

        let ws_root = root.join("workspaces");
        let is_empty = !ws_root.exists() || {
            let mut iter = fs::read_dir(&ws_root).map_err(persist)?;
            iter.next().is_none()
        };

        let store = FsBackend {
            root: root.clone(),
            state: RwLock::new(TreeState::default()),
        };

        if is_empty {
            store.seed_defaults()?;
        }

        // Rebuild index from disk.
        let index = build_index(&root)?;
        let mut state = store.state.write().unwrap();
        state.index = index;
        drop(state);

        Ok(store)
    }

    /// Return the `workspaces/` root.
    fn ws_root(&self) -> PathBuf {
        self.root.join("workspaces")
    }

    /// Read layout file from disk; returns default if file does not exist.
    fn read_layout_file(&self, sess_dir: &Path) -> Result<LayoutFile> {
        let path = sess_dir.join("layout.json");
        if !path.exists() {
            return Ok(LayoutFile::default());
        }
        read_json(&path)
    }

    /// Convert a `ProjectFile` at a given dir into the public `Project` struct.
    /// The workspace id is derived from the directory path.
    fn proj_from_file(&self, proj_dir: &Path) -> Result<(ProjectFile, Project)> {
        let pf: ProjectFile = read_json(&proj_dir.join("project.json"))?;
        let ws_id = ws_id_from_proj_dir(proj_dir)?;
        let project = Project {
            id: ProjectId::new(pf.id.clone()),
            name: pf.name.clone(),
            source_kind: source_kind_from_str(&pf.source_kind)?,
            root_path: pf.root_path.clone(),
            workspace_id: ws_id,
        };
        Ok((pf, project))
    }

    fn sess_from_file(&self, sess_dir: &Path) -> Result<(SessionFile, Session)> {
        let sf: SessionFile = read_json(&sess_dir.join("session.json"))?;
        let project_id = proj_id_from_sess_dir(sess_dir)?;
        let session = Session {
            id: SessionId::from_string(sf.id.clone()),
            project_id,
            title: sf.title.clone(),
            title_source: title_source_from_str(&sf.title_source)?,
            created_at: sf.created_at.clone(),
            spec_version: sf.spec_version,
            spec_json: sf.spec_json.clone(),
        };
        Ok((sf, session))
    }

    /// Seed the Default workspace and Unfiled project (D9).
    fn seed_defaults(&self) -> Result<()> {
        let ws_id = WorkspaceId::default_id();
        let ws_slug = "default";
        let ws_dir = self.ws_root().join(ws_slug);
        create_dir_secure(&ws_dir).map_err(persist)?;

        let wf = WorkspaceFile {
            id: ws_id.as_str().to_owned(),
            name: "Default".to_owned(),
            sort_order: 0,
        };
        atomic_write(&ws_dir.join("workspace.json"), &to_json(&wf)?)?;

        // Unfiled project inside Default workspace.
        let proj_id = ProjectId::unfiled();
        let proj_slug = "unfiled";
        let proj_dir = ws_dir.join("projects").join(proj_slug);
        create_dir_secure(&proj_dir).map_err(persist)?;

        let pf = ProjectFile {
            id: proj_id.as_str().to_owned(),
            name: "Unfiled".to_owned(),
            source_kind: source_kind_str(SourceKind::LocalDir).to_owned(),
            root_path: None,
            sort_order: 0,
        };
        atomic_write(&proj_dir.join("project.json"), &to_json(&pf)?)?;

        Ok(())
    }

    fn dir_sort_order(&self, dir: &Path) -> Option<u32> {
        if let Ok(wf) = read_json::<WorkspaceFile>(&dir.join("workspace.json")) {
            return Some(wf.sort_order);
        }
        if let Ok(pf) = read_json::<ProjectFile>(&dir.join("project.json")) {
            return Some(pf.sort_order);
        }
        if let Ok(sf) = read_json::<SessionFile>(&dir.join("session.json")) {
            return Some(sf.sort_order);
        }
        None
    }

    /// Test-only: read the workspace dir for `id` from the index.
    #[cfg(test)]
    fn ws_dir_for(&self, id: &WorkspaceId) -> Result<PathBuf> {
        let state = self.state.read().unwrap();
        state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::WorkspaceNotFound(id.as_str().to_owned()))
    }

    /// Test-only: resolve a project dir from the index.
    #[cfg(test)]
    fn proj_dir_for(&self, id: &ProjectId) -> Result<PathBuf> {
        let state = self.state.read().unwrap();
        state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(id.as_str().to_owned()))
    }
}

// ── path-based hierarchy helpers ─────────────────────────────────────────────

/// Derive workspace id from a project dir path by walking up until `workspace.json` is found.
fn ws_id_from_proj_dir(proj_dir: &Path) -> Result<WorkspaceId> {
    let mut cur = proj_dir.parent();
    while let Some(p) = cur {
        let ws_file = p.join("workspace.json");
        if ws_file.exists() {
            let wf: WorkspaceFile = read_json(&ws_file)?;
            return Ok(WorkspaceId::new(wf.id));
        }
        cur = p.parent();
    }
    Err(OrchestratorError::Persistence(format!(
        "cannot derive workspace id from {proj_dir:?}"
    )))
}

/// Derive project id from a session dir path by walking up until `project.json` is found.
fn proj_id_from_sess_dir(sess_dir: &Path) -> Result<ProjectId> {
    let mut cur = sess_dir.parent();
    while let Some(p) = cur {
        let pf_path = p.join("project.json");
        if pf_path.exists() {
            let pf: ProjectFile = read_json(&pf_path)?;
            return Ok(ProjectId::new(pf.id));
        }
        cur = p.parent();
    }
    Err(OrchestratorError::Persistence(format!(
        "cannot derive project id from {sess_dir:?}"
    )))
}

// ── index helpers ─────────────────────────────────────────────────────────────

/// After a rename/move, re-walk the subtree at `dir` and update the index for all
/// workspace / project / session ids found there.
fn reindex_subtree(state: &mut TreeState, dir: &Path) -> Result<()> {
    // Is it a workspace dir?
    let ws_file = dir.join("workspace.json");
    if ws_file.exists() {
        let wf: WorkspaceFile = read_json(&ws_file)?;
        state.insert(&wf.id, dir.to_path_buf());
        let proj_root = dir.join("projects");
        if proj_root.exists() {
            for pd in all_dirs_including_archive(&proj_root)? {
                reindex_subtree(state, &pd)?;
            }
        }
        return Ok(());
    }
    // Is it a project dir?
    let pf_path = dir.join("project.json");
    if pf_path.exists() {
        let pf: ProjectFile = read_json(&pf_path)?;
        state.insert(&pf.id, dir.to_path_buf());
        let sess_root = dir.join("sessions");
        if sess_root.exists() {
            for sd in all_dirs_including_archive(&sess_root)? {
                reindex_subtree(state, &sd)?;
            }
        }
        return Ok(());
    }
    // Is it a session dir?
    let sf_path = dir.join("session.json");
    if sf_path.exists() {
        let sf: SessionFile = read_json(&sf_path)?;
        state.insert(&sf.id, dir.to_path_buf());
    }
    Ok(())
}

/// Collect all ids (workspace/project/session) within a subtree for bulk index removal.
fn collect_ids_in_subtree(dir: &Path) -> Result<Vec<String>> {
    let mut ids = Vec::new();
    if let Ok(wf) = read_json::<WorkspaceFile>(&dir.join("workspace.json")) {
        ids.push(wf.id);
    }
    if let Ok(pf) = read_json::<ProjectFile>(&dir.join("project.json")) {
        ids.push(pf.id);
    }
    if let Ok(sf) = read_json::<SessionFile>(&dir.join("session.json")) {
        ids.push(sf.id);
    }
    if dir.is_dir() {
        for entry in fs::read_dir(dir).map_err(persist)? {
            let entry = entry.map_err(persist)?;
            let path = entry.path();
            if path.is_dir() {
                ids.extend(collect_ids_in_subtree(&path)?);
            }
        }
    }
    Ok(ids)
}

// ── name inference ────────────────────────────────────────────────────────────

fn infer_project_name(draft: &NewProject) -> String {
    if let Some(ref root) = draft.root_path {
        let p = std::path::Path::new(root);
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            return name.to_owned();
        }
    }
    "Unnamed Project".to_owned()
}

#[cfg(test)]
mod tests;
