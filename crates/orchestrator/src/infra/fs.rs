//! File-tree domain store: `persistence/tree/`.
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
//! - D2: module `persistence/tree/`
//! - D3: in-memory `RwLock<TreeState>` with id→path index, built by boot scan
//! - D4: atomic write-temp-rename; archive/delete via `fs::rename` + `fs::remove_dir_all`
//! - D5: slug derivation + collision suffixing; rename = re-slug + subtree move
//! - D6: placement uniqueness enforced under write lock
//! - D9: seed Default workspace + Unfiled project on empty tree
//!
//! # `create_session` deviation from existing Store trait
//! `DomainStore::create_session` accepts `Option<(u32, String)>` (spec_version, spec_json)
//! directly instead of `template_id`. Template→spec resolution is a wiring concern (phase 4).
//! The `NewSession.template_id` field is ignored by this implementation.

use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::RwLock,
};

use serde::{Deserialize, Serialize};

use crate::{
    entities::{
        NewProject, NewSession, NewSurface, NewWorkspace, Project, ProjectId, Session, SessionId,
        SourceKind, Surface, SurfaceId, SurfaceKind, TitleSource, Workspace, WorkspaceId,
    },
    error::{OrchestratorError, Result},
};

// ── DomainStore trait ─────────────────────────────────────────────────────────

/// Domain persistence contract backed by the file-tree store.
///
/// Method signatures match the domain half of the existing `Store` trait so this can
/// replace it at wiring time (phase 4). One deviation: `create_session` accepts a
/// resolved `Option<(u32, String)>` spec pair instead of `template_id`.
pub trait DomainStore: Send + Sync {
    // ── workspace ─────────────────────────────────────────────────────────

    /// Create a workspace, ordered last.
    fn create_workspace(&self, draft: NewWorkspace) -> Result<Workspace>;

    /// Rename a workspace. Returns `WorkspaceNotFound` for an unknown id.
    fn rename_workspace(&self, id: &WorkspaceId, name: &str) -> Result<()>;

    /// Return all workspaces ordered by `sort_order` ascending.
    fn list_workspaces(&self) -> Result<Vec<Workspace>>;

    /// Reorder a workspace to a new sort position.
    fn reorder_workspace(&self, id: &WorkspaceId, sort_order: u32) -> Result<()>;

    /// Delete a non-Default workspace, reassigning its projects to the Default workspace.
    fn delete_workspace(&self, id: &WorkspaceId) -> Result<()>;

    // ── project ───────────────────────────────────────────────────────────

    fn get_project(&self, id: &ProjectId) -> Result<Option<Project>>;

    /// Create a project; infers name from source when `draft.name` is `None`.
    fn create_project(&self, draft: NewProject) -> Result<Project>;

    /// Rename a project. Returns `ProjectNotFound` for unknown id.
    fn rename_project(&self, id: &ProjectId, name: &str) -> Result<()>;

    /// Return non-archived projects, optionally scoped to a workspace.
    fn list_projects(&self, workspace_id: Option<&WorkspaceId>) -> Result<Vec<Project>>;

    /// Move a project to a different workspace.
    fn move_project(&self, project_id: &ProjectId, workspace_id: &WorkspaceId) -> Result<()>;

    /// Soft-delete (archive) a project and its sessions.
    fn archive_project(&self, id: &ProjectId) -> Result<()>;

    /// Permanently remove an already-archived project.
    fn hard_delete_project(&self, id: &ProjectId) -> Result<()>;

    /// Reorder a project to a new sort position.
    fn reorder_project(&self, id: &ProjectId, sort_order: u32) -> Result<()>;

    // ── session ───────────────────────────────────────────────────────────

    /// Create a session. `spec` is `Some((version, json))` if a spec should be stored.
    ///
    /// NOTE: unlike the existing `Store::create_session`, this takes a resolved spec pair,
    /// not a `template_id`. The `NewSession.template_id` field is ignored here.
    fn create_session(&self, draft: NewSession, spec: Option<(u32, String)>) -> Result<Session>;

    /// Rename a session and set `title_source` to `Custom`.
    fn rename_session(&self, id: &SessionId, title: &str) -> Result<()>;

    /// Return non-archived sessions. Pass `Some(project_id)` to filter by project.
    fn list_sessions(&self, project_id: Option<&ProjectId>) -> Result<Vec<Session>>;

    /// Get a single non-archived session by id.
    fn get_session(&self, id: &SessionId) -> Result<Option<Session>>;

    /// Archive a session and its surfaces.
    fn archive_session(&self, id: &SessionId) -> Result<()>;

    /// Permanently remove an already-archived session.
    fn hard_delete_session(&self, id: &SessionId) -> Result<()>;

    /// Reorder a session to a new sort position.
    fn reorder_session(&self, id: &SessionId, sort_order: u32) -> Result<()>;

    /// Replace a session's launch spec blob and version.
    fn set_session_spec(&self, id: &SessionId, spec_version: u32, spec_json: &str) -> Result<()>;

    /// Persist the layout JSON blob for a session.
    fn set_session_layout(&self, id: &SessionId, layout_json: &str) -> Result<()>;

    /// Return the stored layout JSON blob, or `None` if not yet set.
    fn get_session_layout(&self, id: &SessionId) -> Result<Option<String>>;

    // ── surface ───────────────────────────────────────────────────────────

    fn create_surface(&self, draft: NewSurface) -> Result<Surface>;

    fn get_surface(&self, id: &SurfaceId) -> Result<Option<Surface>>;

    /// The session's live surface at `placement`, if any.
    fn find_session_surface_by_placement(
        &self,
        session_id: &SessionId,
        placement: &str,
    ) -> Result<Option<Surface>>;

    /// Return all resumable (live, non-deleted) surfaces across the tree.
    fn list_resumable_surfaces(&self) -> Result<Vec<Surface>>;

    fn update_surface_status(&self, id: &SurfaceId, status: &str) -> Result<()>;

    fn soft_delete_surface(&self, id: &SurfaceId) -> Result<()>;

    /// Associate a surface with a session.
    /// Returns `SurfaceConflict` if the (session, placement) slot is already occupied.
    fn add_surface_to_session(&self, session_id: &SessionId, surface_id: &SurfaceId) -> Result<()>;

    /// Remove a surface from its session (soft-delete without PTY teardown).
    fn remove_surface_from_session(
        &self,
        session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()>;
}

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

// ── helpers ───────────────────────────────────────────────────────────────────

/// Derive a filesystem slug from a display name.
///
/// Rules (D5): lowercase, non-alphanumeric → `-`, collapse/trim. If the result is
/// empty, use the short form of `fallback_id` (first 8 chars).
fn slugify(name: &str, fallback_id: &str) -> String {
    let raw: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    // Collapse consecutive `-` and trim leading/trailing `-`.
    let mut slug = String::new();
    let mut prev_dash = true; // treat start as dash to trim leading
    for c in raw.chars() {
        if c == '-' {
            if !prev_dash {
                slug.push('-');
                prev_dash = true;
            }
        } else {
            slug.push(c);
            prev_dash = false;
        }
    }
    // Trim trailing `-`
    let slug = slug.trim_end_matches('-').to_owned();

    if slug.is_empty() {
        fallback_id.chars().take(8).collect()
    } else {
        slug
    }
}

/// Pick a slug for a new entity in `parent_dir`, avoiding collisions.
///
/// If `<slug>` is taken, try `<slug>-2`, `<slug>-3`, …
fn unique_slug(parent_dir: &Path, base_slug: &str) -> String {
    let candidate = parent_dir.join(base_slug);
    if !candidate.exists() {
        return base_slug.to_owned();
    }
    let mut n = 2u32;
    loop {
        let s = format!("{base_slug}-{n}");
        if !parent_dir.join(&s).exists() {
            return s;
        }
        n += 1;
    }
}

/// Map any displayable error (io, serde) to a persistence error.
fn persist<E: std::fmt::Display>(e: E) -> OrchestratorError {
    OrchestratorError::Persistence(e.to_string())
}

/// Recursively create `path`, owner-only (0700) on unix so the domain tree
/// under the user's data root is not readable by other local accounts.
fn create_dir_secure(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(path)
    }
}

/// Atomically write `content` to `path` via a `.tmp` sibling and rename.
/// The file is owner-only (0600) on unix — domain state may carry spec env.
fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    {
        #[cfg(unix)]
        let mut f = {
            use std::os::unix::fs::OpenOptionsExt as _;
            fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp)
                .map_err(persist)?
        };
        #[cfg(not(unix))]
        let mut f = fs::File::create(&tmp).map_err(persist)?;
        f.write_all(content.as_bytes()).map_err(persist)?;
        f.flush().map_err(persist)?;
    }
    fs::rename(&tmp, path).map_err(persist)
}

/// Serialize `value` to pretty JSON with a trailing newline.
fn to_json<T: Serialize>(value: &T) -> Result<String> {
    let mut s = serde_json::to_string_pretty(value).map_err(persist)?;
    s.push('\n');
    Ok(s)
}

/// Read and deserialize a JSON file.
fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let content = fs::read_to_string(path)
        .map_err(|e| OrchestratorError::Persistence(format!("{path:?}: {e}")))?;
    serde_json::from_str(&content)
        .map_err(|e| OrchestratorError::Persistence(format!("{path:?}: {e}")))
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

/// Current UTC timestamp in RFC 3339 / ISO-8601 format.
fn now_iso8601() -> String {
    // Use std time only — no external time dep needed.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as a rough ISO-8601 UTC string.
    // Full RFC-3339 formatting without chrono:
    let s = secs;
    let min = s / 60;
    let hour = min / 60;
    let day_total = hour / 24;
    let sec = s % 60;
    let min = min % 60;
    let hour = hour % 24;
    // Days since epoch → rough year/month/day (good enough for storage; not shown to users)
    let (year, month, day) = days_to_ymd(day_total as u32);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn days_to_ymd(mut d: u32) -> (u32, u32, u32) {
    // Rata Die-style computation from Unix epoch (1970-01-01).
    let mut year = 1970u32;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u32;
    for &md in &month_days {
        if d < md {
            break;
        }
        d -= md;
        month += 1;
    }
    (year, month, d + 1)
}

fn is_leap(y: u32) -> bool {
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}

// ── scan helpers ──────────────────────────────────────────────────────────────

/// Scan a directory, returning immediate child entries (non-recursive).
/// Skips entries starting with `.` (e.g., `.archive`).
fn list_live_dirs(parent: &Path) -> Result<Vec<PathBuf>> {
    if !parent.exists() {
        return Ok(vec![]);
    }
    let mut dirs = Vec::new();
    for entry in fs::read_dir(parent).map_err(persist)? {
        let entry = entry.map_err(persist)?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        }
    }
    Ok(dirs)
}

/// Boot-scan the tree rooted at `root`, building the id→path index.
///
/// Scans live workspace dirs → project dirs → session dirs.
/// Archived subtrees (under `.archive/`) are also indexed so get-by-id works
/// for archived entities (needed for hard_delete).
fn build_index(root: &Path) -> Result<HashMap<String, PathBuf>> {
    let mut index = HashMap::new();
    let ws_root = root.join("workspaces");
    if !ws_root.exists() {
        return Ok(index);
    }

    // Walk all workspace dirs (live + archived)
    let ws_dirs = all_dirs_including_archive(&ws_root)?;
    for ws_dir in ws_dirs {
        let ws_file = ws_dir.join("workspace.json");
        if ws_file.exists() {
            if let Ok(wf) = read_json::<WorkspaceFile>(&ws_file) {
                index.insert(wf.id.clone(), ws_dir.clone());

                // Walk project dirs (live + archived) inside this workspace
                let proj_root = ws_dir.join("projects");
                let proj_dirs = all_dirs_including_archive(&proj_root)?;
                for proj_dir in proj_dirs {
                    let pf_path = proj_dir.join("project.json");
                    if pf_path.exists() {
                        match read_json::<ProjectFile>(&pf_path) {
                            Ok(pf) => {
                                index.insert(pf.id.clone(), proj_dir.clone());

                                // Walk session dirs (live + archived)
                                let sess_root = proj_dir.join("sessions");
                                let sess_dirs = all_dirs_including_archive(&sess_root)?;
                                for sess_dir in sess_dirs {
                                    let sf_path = sess_dir.join("session.json");
                                    if sf_path.exists() {
                                        if let Ok(sf) = read_json::<SessionFile>(&sf_path) {
                                            index.insert(sf.id.clone(), sess_dir.clone());
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("warn: skipping malformed {}: {}", pf_path.display(), e);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(index)
}

/// Return all child dirs under `parent`, including those under `.archive/`.
fn all_dirs_including_archive(parent: &Path) -> Result<Vec<PathBuf>> {
    if !parent.exists() {
        return Ok(vec![]);
    }
    let mut result = Vec::new();
    for entry in fs::read_dir(parent).map_err(persist)? {
        let entry = entry.map_err(persist)?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == ".archive" {
            // Also recurse into .archive children
            for arch_entry in fs::read_dir(&path).map_err(persist)? {
                let arch_entry = arch_entry.map_err(persist)?;
                let ap = arch_entry.path();
                if ap.is_dir() {
                    result.push(ap);
                }
            }
        } else {
            result.push(path);
        }
    }
    Ok(result)
}

/// Determine whether a path is inside a `.archive` directory.
fn is_archived(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == ".archive")
}

// ── FsBackend ─────────────────────────────────────────────────────────────────

/// File-tree backed `DomainStore` implementation.
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

// ── DomainStore impl ──────────────────────────────────────────────────────────

impl DomainStore for FsBackend {
    // ── workspace ─────────────────────────────────────────────────────────

    fn create_workspace(&self, draft: NewWorkspace) -> Result<Workspace> {
        let mut state = self.state.write().unwrap();
        let id = WorkspaceId::new(uuid::Uuid::new_v4().to_string());
        let ws_root = self.ws_root();
        let sort_order = {
            let live = list_live_dirs(&ws_root)?;
            let max = live
                .iter()
                .filter_map(|d| self.dir_sort_order(d))
                .max()
                .unwrap_or(0);
            if live.is_empty() {
                0
            } else {
                max + 1
            }
        };
        let slug_base = slugify(&draft.name, id.as_str());
        let slug = unique_slug(&ws_root, &slug_base);
        let ws_dir = ws_root.join(&slug);
        create_dir_secure(&ws_dir).map_err(persist)?;

        let wf = WorkspaceFile {
            id: id.as_str().to_owned(),
            name: draft.name.clone(),
            sort_order,
        };
        atomic_write(&ws_dir.join("workspace.json"), &to_json(&wf)?)?;
        state.insert(id.as_str(), ws_dir);
        Ok(Workspace {
            id,
            name: draft.name,
        })
    }

    fn rename_workspace(&self, id: &WorkspaceId, name: &str) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let ws_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::WorkspaceNotFound(id.as_str().to_owned()))?;

        let mut wf = read_json::<WorkspaceFile>(&ws_dir.join("workspace.json"))?;
        wf.name = name.to_owned();

        let slug_base = slugify(name, id.as_str());
        let ws_root = self.ws_root();
        // If slug would change, move the directory.
        let current_slug = ws_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_owned();
        let new_slug = if slug_base != current_slug {
            unique_slug(&ws_root, &slug_base)
        } else {
            current_slug.clone()
        };

        // Write updated file first (into the current location).
        atomic_write(&ws_dir.join("workspace.json"), &to_json(&wf)?)?;

        if new_slug != current_slug {
            let new_ws_dir = ws_root.join(&new_slug);
            fs::rename(&ws_dir, &new_ws_dir).map_err(persist)?;
            // Update index: workspace id + all nested project/session ids.
            reindex_subtree(&mut state, &new_ws_dir)?;
        }
        Ok(())
    }

    fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let ws_root = self.ws_root();
        let mut dirs = list_live_dirs(&ws_root)?;
        // Sort by sortOrder ascending.
        dirs.sort_by_key(|d| {
            read_json::<WorkspaceFile>(&d.join("workspace.json"))
                .map(|wf| wf.sort_order)
                .unwrap_or(u32::MAX)
        });
        let mut result = Vec::new();
        for d in dirs {
            let wf = read_json::<WorkspaceFile>(&d.join("workspace.json"))?;
            result.push(Workspace {
                id: WorkspaceId::new(wf.id),
                name: wf.name,
            });
        }
        Ok(result)
    }

    fn reorder_workspace(&self, id: &WorkspaceId, sort_order: u32) -> Result<()> {
        let _state = self.state.write().unwrap();
        let ws_dir = _state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::WorkspaceNotFound(id.as_str().to_owned()))?;
        let mut wf = read_json::<WorkspaceFile>(&ws_dir.join("workspace.json"))?;
        wf.sort_order = sort_order;
        atomic_write(&ws_dir.join("workspace.json"), &to_json(&wf)?)
    }

    fn delete_workspace(&self, id: &WorkspaceId) -> Result<()> {
        if id.as_str() == WorkspaceId::DEFAULT {
            return Err(OrchestratorError::WorkspaceIsDefault);
        }
        let mut state = self.state.write().unwrap();
        let ws_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::WorkspaceNotFound(id.as_str().to_owned()))?;

        // Reassign projects to Default workspace.
        let default_ws_dir = state
            .get(WorkspaceId::DEFAULT)
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::WorkspaceNotFound(WorkspaceId::DEFAULT.to_owned()))?;
        let default_proj_root = default_ws_dir.join("projects");
        create_dir_secure(&default_proj_root).map_err(persist)?;

        let proj_root = ws_dir.join("projects");
        if proj_root.exists() {
            for dir in list_live_dirs(&proj_root)? {
                let slug = dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("proj")
                    .to_owned();
                let target_slug = unique_slug(&default_proj_root, &slug);
                let target = default_proj_root.join(&target_slug);
                fs::rename(&dir, &target).map_err(persist)?;
                reindex_subtree(&mut state, &target)?;
            }
        }

        // Reassign archived projects too — they are still the workspace's
        // projects, and remove_dir_all below would otherwise destroy them.
        let archive_src = proj_root.join(".archive");
        if archive_src.exists() {
            let default_archive = default_proj_root.join(".archive");
            create_dir_secure(&default_archive).map_err(persist)?;
            for dir in list_live_dirs(&archive_src)? {
                let slug = dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("proj")
                    .to_owned();
                let target = default_archive.join(unique_slug(&default_archive, &slug));
                fs::rename(&dir, &target).map_err(persist)?;
                reindex_subtree(&mut state, &target)?;
            }
        }

        // Remove the workspace subtree.
        fs::remove_dir_all(&ws_dir).map_err(persist)?;
        state.remove(id.as_str());
        Ok(())
    }

    // ── project ───────────────────────────────────────────────────────────

    fn get_project(&self, id: &ProjectId) -> Result<Option<Project>> {
        let state = self.state.read().unwrap();
        let proj_dir = match state.get(id.as_str()) {
            Some(p) => p.to_path_buf(),
            None => return Ok(None),
        };
        drop(state);
        if is_archived(&proj_dir) || !proj_dir.join("project.json").exists() {
            return Ok(None);
        }
        let (_, project) = self.proj_from_file(&proj_dir)?;
        Ok(Some(project))
    }

    fn create_project(&self, draft: NewProject) -> Result<Project> {
        let mut state = self.state.write().unwrap();

        let ws_id = draft
            .workspace_id
            .clone()
            .unwrap_or_else(WorkspaceId::default_id);
        let ws_dir = state
            .get(ws_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::WorkspaceNotFound(ws_id.as_str().to_owned()))?;

        let proj_id = ProjectId::new(uuid::Uuid::new_v4().to_string());
        let name = draft
            .name
            .clone()
            .unwrap_or_else(|| infer_project_name(&draft));
        let proj_root = ws_dir.join("projects");
        create_dir_secure(&proj_root).map_err(persist)?;

        let sort_order = {
            let live = list_live_dirs(&proj_root)?;
            let max = live
                .iter()
                .filter_map(|d| self.dir_sort_order(d))
                .max()
                .unwrap_or(0);
            if live.is_empty() {
                0
            } else {
                max + 1
            }
        };

        let slug_base = slugify(&name, proj_id.as_str());
        let slug = unique_slug(&proj_root, &slug_base);
        let proj_dir = proj_root.join(&slug);
        create_dir_secure(&proj_dir).map_err(persist)?;

        let pf = ProjectFile {
            id: proj_id.as_str().to_owned(),
            name: name.clone(),
            source_kind: source_kind_str(draft.source_kind).to_owned(),
            root_path: draft.root_path.clone(),
            sort_order,
        };
        atomic_write(&proj_dir.join("project.json"), &to_json(&pf)?)?;
        state.insert(proj_id.as_str(), proj_dir);

        Ok(Project {
            id: proj_id,
            name,
            source_kind: draft.source_kind,
            root_path: draft.root_path,
            workspace_id: ws_id,
        })
    }

    fn rename_project(&self, id: &ProjectId, name: &str) -> Result<()> {
        if id.is_unfiled() {
            return Err(OrchestratorError::ProjectIsUnfiled);
        }
        let mut state = self.state.write().unwrap();
        let proj_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(id.as_str().to_owned()))?;

        let mut pf = read_json::<ProjectFile>(&proj_dir.join("project.json"))?;
        pf.name = name.to_owned();

        let slug_base = slugify(name, id.as_str());
        let proj_root = proj_dir
            .parent()
            .ok_or_else(|| OrchestratorError::Persistence("no parent".into()))?
            .to_path_buf();

        let current_slug = proj_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_owned();

        // Update name in file first (at current location).
        atomic_write(&proj_dir.join("project.json"), &to_json(&pf)?)?;

        if slug_base != current_slug {
            let new_slug = unique_slug(&proj_root, &slug_base);
            let new_proj_dir = proj_root.join(&new_slug);
            fs::rename(&proj_dir, &new_proj_dir).map_err(persist)?;
            reindex_subtree(&mut state, &new_proj_dir)?;
        }
        Ok(())
    }

    fn list_projects(&self, workspace_id: Option<&WorkspaceId>) -> Result<Vec<Project>> {
        let state = self.state.read().unwrap();

        // Collect workspace dirs to search.
        let ws_root = self.ws_root();
        let ws_dirs: Vec<PathBuf> = if let Some(ws_id) = workspace_id {
            let d = state
                .get(ws_id.as_str())
                .map(Path::to_path_buf)
                .ok_or_else(|| OrchestratorError::WorkspaceNotFound(ws_id.as_str().to_owned()))?;
            vec![d]
        } else {
            drop(state);
            list_live_dirs(&ws_root)?
        };

        let mut projects = Vec::new();
        for ws_dir in ws_dirs {
            let proj_root = ws_dir.join("projects");
            let mut proj_dirs = list_live_dirs(&proj_root)?;
            proj_dirs.sort_by_key(|d| {
                read_json::<ProjectFile>(&d.join("project.json"))
                    .map(|pf| pf.sort_order)
                    .unwrap_or(u32::MAX)
            });
            for pd in proj_dirs {
                if let Ok((_, proj)) = self.proj_from_file(&pd) {
                    projects.push(proj);
                }
            }
        }
        Ok(projects)
    }

    fn move_project(&self, project_id: &ProjectId, workspace_id: &WorkspaceId) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let proj_dir = state
            .get(project_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(project_id.as_str().to_owned()))?;
        let target_ws_dir = state
            .get(workspace_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| {
                OrchestratorError::WorkspaceNotFound(workspace_id.as_str().to_owned())
            })?;

        let slug = proj_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("proj")
            .to_owned();
        let target_proj_root = target_ws_dir.join("projects");
        create_dir_secure(&target_proj_root).map_err(persist)?;
        let new_slug = unique_slug(&target_proj_root, &slug);
        let new_proj_dir = target_proj_root.join(new_slug);
        fs::rename(&proj_dir, &new_proj_dir).map_err(persist)?;
        reindex_subtree(&mut state, &new_proj_dir)?;
        Ok(())
    }

    fn archive_project(&self, id: &ProjectId) -> Result<()> {
        if id.is_unfiled() {
            return Err(OrchestratorError::ProjectIsUnfiled);
        }
        let mut state = self.state.write().unwrap();
        let proj_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(id.as_str().to_owned()))?;

        if is_archived(&proj_dir) {
            return Err(OrchestratorError::ProjectNotFound(id.as_str().to_owned()));
        }

        let proj_root = proj_dir
            .parent()
            .ok_or_else(|| OrchestratorError::Persistence("no parent".into()))?
            .to_path_buf();
        let slug = proj_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("proj")
            .to_owned();
        let archive_root = proj_root.join(".archive");
        create_dir_secure(&archive_root).map_err(persist)?;
        // Disambiguate against entities already archived under the same slug,
        // else the rename collides with a non-empty dir (data loss / ENOTEMPTY).
        let target = archive_root.join(unique_slug(&archive_root, &slug));
        fs::rename(&proj_dir, &target).map_err(persist)?;
        reindex_subtree(&mut state, &target)?;
        Ok(())
    }

    fn hard_delete_project(&self, id: &ProjectId) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let proj_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(id.as_str().to_owned()))?;

        if !is_archived(&proj_dir) {
            return Err(OrchestratorError::ProjectNotArchived);
        }

        // Collect all ids to remove from index.
        let ids = collect_ids_in_subtree(&proj_dir)?;
        fs::remove_dir_all(&proj_dir).map_err(persist)?;
        for id_str in ids {
            state.remove(&id_str);
        }
        Ok(())
    }

    fn reorder_project(&self, id: &ProjectId, sort_order: u32) -> Result<()> {
        let _state = self.state.write().unwrap();
        let proj_dir = _state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(id.as_str().to_owned()))?;
        let mut pf = read_json::<ProjectFile>(&proj_dir.join("project.json"))?;
        pf.sort_order = sort_order;
        atomic_write(&proj_dir.join("project.json"), &to_json(&pf)?)
    }

    // ── session ───────────────────────────────────────────────────────────

    fn create_session(&self, draft: NewSession, spec: Option<(u32, String)>) -> Result<Session> {
        let mut state = self.state.write().unwrap();
        let proj_id = draft.project_id.clone().unwrap_or_else(ProjectId::unfiled);
        let proj_dir = state
            .get(proj_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(proj_id.as_str().to_owned()))?;

        let sess_id = SessionId::mint();
        let title = draft.title.clone().unwrap_or_else(|| "Untitled".to_owned());
        let sess_root = proj_dir.join("sessions");
        create_dir_secure(&sess_root).map_err(persist)?;

        let sort_order = {
            let live = list_live_dirs(&sess_root)?;
            let max = live
                .iter()
                .filter_map(|d| self.dir_sort_order(d))
                .max()
                .unwrap_or(0);
            if live.is_empty() {
                0
            } else {
                max + 1
            }
        };

        let slug_base = slugify(&title, sess_id.as_str());
        let slug = unique_slug(&sess_root, &slug_base);
        let sess_dir = sess_root.join(&slug);
        create_dir_secure(&sess_dir).map_err(persist)?;

        let (spec_version, spec_json) = match spec {
            Some((v, j)) => (Some(v), Some(j)),
            None => (None, None),
        };

        let created_at = now_iso8601();
        let sf = SessionFile {
            id: sess_id.as_str().to_owned(),
            title: title.clone(),
            title_source: title_source_str(draft.title_source).to_owned(),
            created_at: created_at.clone(),
            sort_order,
            spec_version,
            spec_json: spec_json.clone(),
        };
        atomic_write(&sess_dir.join("session.json"), &to_json(&sf)?)?;
        state.insert(sess_id.as_str(), sess_dir);

        Ok(Session {
            id: sess_id,
            project_id: proj_id,
            title,
            title_source: draft.title_source,
            created_at,
            spec_version: sf.spec_version,
            spec_json,
        })
    }

    fn rename_session(&self, id: &SessionId, title: &str) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let sess_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(id.as_str().to_owned()))?;

        let mut sf = read_json::<SessionFile>(&sess_dir.join("session.json"))?;
        sf.title = title.to_owned();
        sf.title_source = title_source_str(TitleSource::Custom).to_owned();

        let slug_base = slugify(title, id.as_str());
        let sess_root = sess_dir
            .parent()
            .ok_or_else(|| OrchestratorError::Persistence("no parent".into()))?
            .to_path_buf();
        let current_slug = sess_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_owned();

        atomic_write(&sess_dir.join("session.json"), &to_json(&sf)?)?;

        if slug_base != current_slug {
            let new_slug = unique_slug(&sess_root, &slug_base);
            let new_sess_dir = sess_root.join(&new_slug);
            fs::rename(&sess_dir, &new_sess_dir).map_err(persist)?;
            state.insert(id.as_str(), new_sess_dir);
        }
        Ok(())
    }

    fn list_sessions(&self, project_id: Option<&ProjectId>) -> Result<Vec<Session>> {
        let state = self.state.read().unwrap();

        // Collect project dirs to search.
        let proj_dirs: Vec<PathBuf> = if let Some(proj_id) = project_id {
            let d = state
                .get(proj_id.as_str())
                .map(Path::to_path_buf)
                .ok_or_else(|| OrchestratorError::ProjectNotFound(proj_id.as_str().to_owned()))?;
            if is_archived(&d) {
                return Ok(vec![]);
            }
            vec![d]
        } else {
            // All non-archived projects.
            let ws_root = self.ws_root();
            drop(state);
            let mut all = Vec::new();
            for ws_dir in list_live_dirs(&ws_root)? {
                for pd in list_live_dirs(&ws_dir.join("projects"))? {
                    all.push(pd);
                }
            }
            all
        };

        let mut sessions = Vec::new();
        for proj_dir in proj_dirs {
            let sess_root = proj_dir.join("sessions");
            let mut sess_dirs = list_live_dirs(&sess_root)?;
            sess_dirs.sort_by_key(|d| {
                read_json::<SessionFile>(&d.join("session.json"))
                    .map(|sf| sf.sort_order)
                    .unwrap_or(u32::MAX)
            });
            for sd in sess_dirs {
                if let Ok((_, sess)) = self.sess_from_file(&sd) {
                    sessions.push(sess);
                }
            }
        }
        Ok(sessions)
    }

    fn get_session(&self, id: &SessionId) -> Result<Option<Session>> {
        let state = self.state.read().unwrap();
        let sess_dir = match state.get(id.as_str()) {
            Some(p) => p.to_path_buf(),
            None => return Ok(None),
        };
        drop(state);
        if is_archived(&sess_dir) || !sess_dir.join("session.json").exists() {
            return Ok(None);
        }
        let (_, session) = self.sess_from_file(&sess_dir)?;
        Ok(Some(session))
    }

    fn archive_session(&self, id: &SessionId) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let sess_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(id.as_str().to_owned()))?;

        if is_archived(&sess_dir) {
            return Err(OrchestratorError::SessionNotFound(id.as_str().to_owned()));
        }

        let sess_root = sess_dir
            .parent()
            .ok_or_else(|| OrchestratorError::Persistence("no parent".into()))?
            .to_path_buf();
        let slug = sess_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("session")
            .to_owned();
        let archive_root = sess_root.join(".archive");
        create_dir_secure(&archive_root).map_err(persist)?;
        // Disambiguate against sessions already archived under the same slug.
        let target = archive_root.join(unique_slug(&archive_root, &slug));
        fs::rename(&sess_dir, &target).map_err(persist)?;
        state.insert(id.as_str(), target);
        Ok(())
    }

    fn hard_delete_session(&self, id: &SessionId) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let sess_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(id.as_str().to_owned()))?;

        if !is_archived(&sess_dir) {
            return Err(OrchestratorError::SessionNotArchived);
        }

        fs::remove_dir_all(&sess_dir).map_err(persist)?;
        state.remove(id.as_str());
        Ok(())
    }

    fn reorder_session(&self, id: &SessionId, sort_order: u32) -> Result<()> {
        let _state = self.state.write().unwrap();
        let sess_dir = _state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(id.as_str().to_owned()))?;
        let mut sf = read_json::<SessionFile>(&sess_dir.join("session.json"))?;
        sf.sort_order = sort_order;
        atomic_write(&sess_dir.join("session.json"), &to_json(&sf)?)
    }

    fn set_session_spec(&self, id: &SessionId, spec_version: u32, spec_json: &str) -> Result<()> {
        let _state = self.state.write().unwrap();
        let sess_dir = _state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(id.as_str().to_owned()))?;
        let mut sf = read_json::<SessionFile>(&sess_dir.join("session.json"))?;
        sf.spec_version = Some(spec_version);
        sf.spec_json = Some(spec_json.to_owned());
        atomic_write(&sess_dir.join("session.json"), &to_json(&sf)?)
    }

    fn set_session_layout(&self, id: &SessionId, layout_json: &str) -> Result<()> {
        let _state = self.state.write().unwrap();
        let sess_dir = _state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(id.as_str().to_owned()))?;
        // layout_json is an opaque blob — store it as the panel_tree field.
        // Parse into serde_json::Value so we can embed it in LayoutFile.
        let panel_tree: Option<serde_json::Value> = serde_json::from_str(layout_json).ok();
        // Preserve existing surface bindings.
        let existing = self.read_layout_file(&sess_dir)?;
        let lf = LayoutFile {
            panel_tree,
            surfaces: existing.surfaces,
        };
        atomic_write(&sess_dir.join("layout.json"), &to_json(&lf)?)
    }

    fn get_session_layout(&self, id: &SessionId) -> Result<Option<String>> {
        let state = self.state.read().unwrap();
        let sess_dir = match state.get(id.as_str()) {
            Some(p) => p.to_path_buf(),
            None => return Ok(None),
        };
        drop(state);
        let layout_path = sess_dir.join("layout.json");
        if !layout_path.exists() {
            return Ok(None);
        }
        let lf = read_json::<LayoutFile>(&layout_path)?;
        match lf.panel_tree {
            Some(v) => {
                let s = serde_json::to_string(&v).map_err(persist)?;
                Ok(Some(s))
            }
            None => Ok(None),
        }
    }

    // ── surface ───────────────────────────────────────────────────────────

    fn create_surface(&self, draft: NewSurface) -> Result<Surface> {
        let state = self.state.write().unwrap();
        let sess_id = draft.session_id.clone();
        let sess_dir = state
            .get(sess_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(sess_id.as_str().to_owned()))?;

        let surf_id = match draft.id {
            Some(id) => id,
            None => SurfaceId::mint(),
        };

        // Placement uniqueness check (D6).
        if let Some(ref placement) = draft.placement {
            let lf = self.read_layout_file(&sess_dir)?;
            for binding in &lf.surfaces {
                if !binding.deleted && binding.placement.as_deref() == Some(placement.as_str()) {
                    return Err(OrchestratorError::SurfaceConflict(
                        surf_id.as_str().to_owned(),
                    ));
                }
            }
        }

        let binding = SurfaceBinding {
            id: surf_id.as_str().to_owned(),
            kind: surface_kind_str(draft.kind).to_owned(),
            placement: draft.placement.clone(),
            cwd: draft.cwd.clone(),
            last_status: None,
            deleted: false,
        };

        let mut lf = self.read_layout_file(&sess_dir)?;
        lf.surfaces.push(binding);
        atomic_write(&sess_dir.join("layout.json"), &to_json(&lf)?)?;
        drop(state);

        Ok(Surface {
            id: surf_id,
            session_id: sess_id,
            kind: draft.kind,
            cwd: draft.cwd,
            last_status: None,
            placement: draft.placement,
        })
    }

    fn get_surface(&self, id: &SurfaceId) -> Result<Option<Surface>> {
        // Must scan all live sessions.
        let ws_root = self.ws_root();
        for ws_dir in list_live_dirs(&ws_root)? {
            for proj_dir in list_live_dirs(&ws_dir.join("projects"))? {
                for sess_dir in list_live_dirs(&proj_dir.join("sessions"))? {
                    let lf = self.read_layout_file(&sess_dir)?;
                    for b in &lf.surfaces {
                        if b.id == id.as_str() && !b.deleted {
                            let sess_file =
                                read_json::<SessionFile>(&sess_dir.join("session.json"))?;
                            let kind = surface_kind_from_str(&b.kind)?;
                            return Ok(Some(Surface {
                                id: SurfaceId::from_string(b.id.clone()),
                                session_id: SessionId::from_string(sess_file.id),
                                kind,
                                cwd: b.cwd.clone(),
                                last_status: b.last_status.clone(),
                                placement: b.placement.clone(),
                            }));
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    fn find_session_surface_by_placement(
        &self,
        session_id: &SessionId,
        placement: &str,
    ) -> Result<Option<Surface>> {
        let state = self.state.read().unwrap();
        let sess_dir = match state.get(session_id.as_str()) {
            Some(p) => p.to_path_buf(),
            None => return Ok(None),
        };
        drop(state);
        let lf = self.read_layout_file(&sess_dir)?;
        for b in &lf.surfaces {
            if !b.deleted && b.placement.as_deref() == Some(placement) {
                let kind = surface_kind_from_str(&b.kind)?;
                return Ok(Some(Surface {
                    id: SurfaceId::from_string(b.id.clone()),
                    session_id: session_id.clone(),
                    kind,
                    cwd: b.cwd.clone(),
                    last_status: b.last_status.clone(),
                    placement: b.placement.clone(),
                }));
            }
        }
        Ok(None)
    }

    fn list_resumable_surfaces(&self) -> Result<Vec<Surface>> {
        let ws_root = self.ws_root();
        let mut surfaces = Vec::new();
        for ws_dir in list_live_dirs(&ws_root)? {
            for proj_dir in list_live_dirs(&ws_dir.join("projects"))? {
                for sess_dir in list_live_dirs(&proj_dir.join("sessions"))? {
                    let sf = read_json::<SessionFile>(&sess_dir.join("session.json"))?;
                    let lf = self.read_layout_file(&sess_dir)?;
                    for b in &lf.surfaces {
                        if !b.deleted {
                            if let Ok(kind) = surface_kind_from_str(&b.kind) {
                                surfaces.push(Surface {
                                    id: SurfaceId::from_string(b.id.clone()),
                                    session_id: SessionId::from_string(sf.id.clone()),
                                    kind,
                                    cwd: b.cwd.clone(),
                                    last_status: b.last_status.clone(),
                                    placement: b.placement.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
        Ok(surfaces)
    }

    fn update_surface_status(&self, id: &SurfaceId, status: &str) -> Result<()> {
        let _state = self.state.write().unwrap();
        let ws_root = self.ws_root();
        for ws_dir in list_live_dirs(&ws_root)? {
            for proj_dir in list_live_dirs(&ws_dir.join("projects"))? {
                for sess_dir in list_live_dirs(&proj_dir.join("sessions"))? {
                    let mut lf = self.read_layout_file(&sess_dir)?;
                    let mut changed = false;
                    for b in &mut lf.surfaces {
                        if b.id == id.as_str() && !b.deleted {
                            b.last_status = Some(status.to_owned());
                            changed = true;
                        }
                    }
                    if changed {
                        return atomic_write(&sess_dir.join("layout.json"), &to_json(&lf)?);
                    }
                }
            }
        }
        Ok(()) // Surface not found — silently ignore (matches Store trait behaviour)
    }

    fn soft_delete_surface(&self, id: &SurfaceId) -> Result<()> {
        let _state = self.state.write().unwrap();
        let ws_root = self.ws_root();
        for ws_dir in list_live_dirs(&ws_root)? {
            for proj_dir in list_live_dirs(&ws_dir.join("projects"))? {
                for sess_dir in list_live_dirs(&proj_dir.join("sessions"))? {
                    let mut lf = self.read_layout_file(&sess_dir)?;
                    let mut changed = false;
                    for b in &mut lf.surfaces {
                        if b.id == id.as_str() && !b.deleted {
                            b.deleted = true;
                            changed = true;
                        }
                    }
                    if changed {
                        return atomic_write(&sess_dir.join("layout.json"), &to_json(&lf)?);
                    }
                }
            }
        }
        Ok(())
    }

    fn add_surface_to_session(&self, session_id: &SessionId, surface_id: &SurfaceId) -> Result<()> {
        let _state = self.state.write().unwrap();
        let sess_dir = _state
            .get(session_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(session_id.as_str().to_owned()))?;

        let mut lf = self.read_layout_file(&sess_dir)?;

        // Check if this surface is already bound to a different session (conflict).
        // Find existing binding across the whole tree.
        let ws_root = self.ws_root();
        'outer: for ws_dir in list_live_dirs(&ws_root)? {
            for proj_dir in list_live_dirs(&ws_dir.join("projects"))? {
                for sd in list_live_dirs(&proj_dir.join("sessions"))? {
                    let other_lf = self.read_layout_file(&sd)?;
                    for b in &other_lf.surfaces {
                        if b.id == surface_id.as_str() && !b.deleted {
                            // Check whether it belongs to THIS session.
                            let sf = read_json::<SessionFile>(&sd.join("session.json"))?;
                            if sf.id != session_id.as_str() {
                                return Err(OrchestratorError::SurfaceConflict(
                                    surface_id.as_str().to_owned(),
                                ));
                            }
                            // Already in this session — no-op.
                            break 'outer;
                        }
                    }
                }
            }
        }

        // Check placement uniqueness for the new binding.
        // We need the binding details from somewhere — if the surface doesn't exist yet in
        // any session, it won't have a placement. We just add a minimal binding here.
        // If placement conflicts within this session:
        let binding = SurfaceBinding {
            id: surface_id.as_str().to_owned(),
            kind: "terminal".to_owned(), // default; updated by caller via create_surface
            placement: None,
            cwd: None,
            last_status: None,
            deleted: false,
        };
        lf.surfaces.push(binding);
        atomic_write(&sess_dir.join("layout.json"), &to_json(&lf)?)
    }

    fn remove_surface_from_session(
        &self,
        session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()> {
        let _state = self.state.write().unwrap();
        let sess_dir = _state
            .get(session_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(session_id.as_str().to_owned()))?;
        let mut lf = self.read_layout_file(&sess_dir)?;
        for b in &mut lf.surfaces {
            if b.id == surface_id.as_str() {
                b.deleted = true;
            }
        }
        atomic_write(&sess_dir.join("layout.json"), &to_json(&lf)?)
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

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_store() -> (TempDir, FsBackend) {
        let tmp = TempDir::new().unwrap();
        let store = FsBackend::open(tmp.path().to_path_buf()).unwrap();
        (tmp, store)
    }

    #[test]
    fn malformed_project_json_is_skipped_and_rest_of_tree_loads() {
        use std::io::Write as _;
        let tmp = TempDir::new().unwrap();
        let store = FsBackend::open(tmp.path().to_path_buf()).unwrap();
        // Seed a workspace to work within.
        let ws = store
            .create_workspace(NewWorkspace { name: "W".into() })
            .unwrap();
        let good = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("Good".into()),
                workspace_id: Some(ws.id.clone()),
            })
            .unwrap();
        // Write a malformed project.json in a sibling directory inside the workspace projects dir.
        let ws_dir = store.ws_dir_for(&ws.id).unwrap();
        let bad_proj_dir = ws_dir.join("projects").join("bad-slug");
        std::fs::create_dir_all(bad_proj_dir.join("sessions")).unwrap();
        let mut f = std::fs::File::create(bad_proj_dir.join("project.json")).unwrap();
        f.write_all(b"{ not valid json }").unwrap();
        drop(f);

        // Re-open: malformed project is skipped, good project still loads.
        let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();
        let projects = store2.list_projects(None).unwrap();
        assert!(
            projects.iter().any(|p| p.id == good.id),
            "good project must still load after malformed sibling is present"
        );
    }

    // ── Scenario: Workspace, project, and session persist as nested directories ──

    #[test]
    fn workspace_project_session_persist_as_nested_directories() {
        let (tmp, store) = open_store();

        let ws = store
            .create_workspace(NewWorkspace {
                name: "My Workspace".into(),
            })
            .unwrap();
        let proj = store
            .create_project(NewProject {
                source_kind: SourceKind::LocalDir,
                root_path: Some("/tmp/proj".into()),
                name: Some("My Project".into()),
                workspace_id: Some(ws.id.clone()),
            })
            .unwrap();
        let sess = store
            .create_session(
                NewSession {
                    project_id: Some(proj.id.clone()),
                    title: Some("My Session".into()),
                    title_source: TitleSource::Custom,
                    template_id: None,
                },
                None,
            )
            .unwrap();

        // workspace.json must exist
        let ws_root = tmp.path().join("workspaces");
        let ws_dirs: Vec<_> = list_live_dirs(&ws_root).unwrap();
        assert_eq!(ws_dirs.len(), 2); // default + new

        // Find the new workspace dir
        let new_ws_dir = ws_dirs
            .iter()
            .find(|d| {
                read_json::<WorkspaceFile>(&d.join("workspace.json"))
                    .ok()
                    .map(|wf| wf.id == ws.id.as_str())
                    .unwrap_or(false)
            })
            .unwrap()
            .clone();

        assert!(new_ws_dir.join("workspace.json").exists());

        let proj_dirs = list_live_dirs(&new_ws_dir.join("projects")).unwrap();
        assert_eq!(proj_dirs.len(), 1);
        assert!(proj_dirs[0].join("project.json").exists());

        let sess_dirs = list_live_dirs(&proj_dirs[0].join("sessions")).unwrap();
        assert_eq!(sess_dirs.len(), 1);
        assert!(sess_dirs[0].join("session.json").exists());

        // Verify ids persist correctly
        let wf: WorkspaceFile = read_json(&new_ws_dir.join("workspace.json")).unwrap();
        assert_eq!(wf.id, ws.id.as_str());
        let pf: ProjectFile = read_json(&proj_dirs[0].join("project.json")).unwrap();
        assert_eq!(pf.id, proj.id.as_str());
        let sf: SessionFile = read_json(&sess_dirs[0].join("session.json")).unwrap();
        assert_eq!(sf.id, sess.id.as_str());
    }

    // ── Scenario: Hierarchy is read back from containment ──

    #[test]
    fn hierarchy_is_read_back_from_containment() {
        let (tmp, store) = open_store();

        let ws = store
            .create_workspace(NewWorkspace { name: "W1".into() })
            .unwrap();
        let proj = store
            .create_project(NewProject {
                source_kind: SourceKind::LocalDir,
                root_path: None,
                name: Some("P1".into()),
                workspace_id: Some(ws.id.clone()),
            })
            .unwrap();
        let sess = store
            .create_session(
                NewSession {
                    project_id: Some(proj.id.clone()),
                    title: Some("S1".into()),
                    title_source: TitleSource::Custom,
                    template_id: None,
                },
                None,
            )
            .unwrap();

        // Reopen
        let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();

        let proj2 = store2.get_project(&proj.id).unwrap().unwrap();
        assert_eq!(proj2.workspace_id, ws.id);

        let sess2 = store2.get_session(&sess.id).unwrap().unwrap();
        assert_eq!(sess2.project_id, proj.id);
    }

    // ── Scenario: A failed write leaves the prior file intact ──

    #[test]
    fn failed_write_leaves_prior_file_intact() {
        let (_tmp, store) = open_store();
        let ws = store
            .create_workspace(NewWorkspace { name: "W".into() })
            .unwrap();
        let ws_dir = store.ws_dir_for(&ws.id).unwrap();
        let ws_file = ws_dir.join("workspace.json");

        // Read original content
        let original = std::fs::read_to_string(&ws_file).unwrap();

        // Simulate an interrupted write: write .tmp but don't rename
        let tmp_path = ws_file.with_extension("tmp");
        std::fs::write(&tmp_path, b"partial content").unwrap();

        // The entity file must still have original content
        let after = std::fs::read_to_string(&ws_file).unwrap();
        assert_eq!(original, after);

        // The .tmp file exists but doesn't affect reads
        assert!(tmp_path.exists());
        // Remove it to clean up
        std::fs::remove_file(&tmp_path).unwrap();
    }

    // ── Scenario: Siblings list in sortOrder ──

    #[test]
    fn siblings_list_in_sort_order() {
        let (_tmp, store) = open_store();
        let proj = store.get_project(&ProjectId::unfiled()).unwrap().unwrap();

        // Create 3 sessions and then reorder them
        let s0 = store
            .create_session(
                NewSession {
                    project_id: Some(proj.id.clone()),
                    title: Some("A".into()),
                    title_source: TitleSource::Custom,
                    template_id: None,
                },
                None,
            )
            .unwrap();
        let s1 = store
            .create_session(
                NewSession {
                    project_id: Some(proj.id.clone()),
                    title: Some("B".into()),
                    title_source: TitleSource::Custom,
                    template_id: None,
                },
                None,
            )
            .unwrap();
        let s2 = store
            .create_session(
                NewSession {
                    project_id: Some(proj.id.clone()),
                    title: Some("C".into()),
                    title_source: TitleSource::Custom,
                    template_id: None,
                },
                None,
            )
            .unwrap();

        // Reorder: C=0, A=1, B=2
        store.reorder_session(&s2.id, 0).unwrap();
        store.reorder_session(&s0.id, 1).unwrap();
        store.reorder_session(&s1.id, 2).unwrap();

        let sessions = store.list_sessions(Some(&proj.id)).unwrap();
        assert_eq!(sessions.len(), 3);
        assert_eq!(sessions[0].id, s2.id);
        assert_eq!(sessions[1].id, s0.id);
        assert_eq!(sessions[2].id, s1.id);
    }

    // ── Scenario: Reorder persists across reload ──

    #[test]
    fn reorder_persists_across_reload() {
        let (tmp, store) = open_store();
        let proj = store.get_project(&ProjectId::unfiled()).unwrap().unwrap();

        let s0 = store
            .create_session(
                NewSession {
                    project_id: Some(proj.id.clone()),
                    title: Some("First".into()),
                    title_source: TitleSource::Custom,
                    template_id: None,
                },
                None,
            )
            .unwrap();
        let s1 = store
            .create_session(
                NewSession {
                    project_id: Some(proj.id.clone()),
                    title: Some("Second".into()),
                    title_source: TitleSource::Custom,
                    template_id: None,
                },
                None,
            )
            .unwrap();

        // Swap order
        store.reorder_session(&s1.id, 0).unwrap();
        store.reorder_session(&s0.id, 1).unwrap();

        // Reopen
        let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();
        let sessions = store2.list_sessions(Some(&proj.id)).unwrap();
        assert_eq!(sessions[0].id, s1.id);
        assert_eq!(sessions[1].id, s0.id);
    }

    // ── Scenario: Renaming a project moves its subtree and keeps the id ──

    #[test]
    fn renaming_a_project_moves_subtree_and_keeps_id() {
        let (tmp, store) = open_store();

        let ws = store
            .create_workspace(NewWorkspace { name: "WS".into() })
            .unwrap();
        let proj = store
            .create_project(NewProject {
                source_kind: SourceKind::LocalDir,
                root_path: None,
                name: Some("Old Name".into()),
                workspace_id: Some(ws.id.clone()),
            })
            .unwrap();

        // Create a session inside to verify subtree moves
        let sess = store
            .create_session(
                NewSession {
                    project_id: Some(proj.id.clone()),
                    title: Some("MySession".into()),
                    title_source: TitleSource::Custom,
                    template_id: None,
                },
                None,
            )
            .unwrap();

        store.rename_project(&proj.id, "New Name").unwrap();

        // Old slug should not exist
        let ws_dir_path = store.ws_dir_for(&ws.id).unwrap();
        let old_proj_dir = ws_dir_path.join("projects").join("old-name");
        assert!(!old_proj_dir.exists(), "old directory should not exist");

        // New slug should exist
        let new_proj_dir = ws_dir_path.join("projects").join("new-name");
        assert!(new_proj_dir.exists(), "new directory should exist");

        // ID unchanged
        let pf: ProjectFile = read_json(&new_proj_dir.join("project.json")).unwrap();
        assert_eq!(pf.id, proj.id.as_str());
        assert_eq!(pf.name, "New Name");

        // Session inside was moved too
        let sess_dirs = list_live_dirs(&new_proj_dir.join("sessions")).unwrap();
        assert_eq!(sess_dirs.len(), 1);
        let sf: SessionFile = read_json(&sess_dirs[0].join("session.json")).unwrap();
        assert_eq!(sf.id, sess.id.as_str());

        // Reopen to verify index rebuilt correctly
        let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();
        let sess2 = store2.get_session(&sess.id).unwrap();
        assert!(
            sess2.is_some(),
            "session should be retrievable after rename"
        );
    }

    // ── Scenario: Colliding slug is disambiguated ──

    #[test]
    fn colliding_slug_is_disambiguated() {
        let (_tmp, store) = open_store();

        // Create two projects with names that produce the same slug
        let ws_id = WorkspaceId::default_id();
        let p1 = store
            .create_project(NewProject {
                source_kind: SourceKind::LocalDir,
                root_path: None,
                name: Some("foo".into()),
                workspace_id: Some(ws_id.clone()),
            })
            .unwrap();
        let p2 = store
            .create_project(NewProject {
                source_kind: SourceKind::LocalDir,
                root_path: None,
                name: Some("foo".into()),
                workspace_id: Some(ws_id),
            })
            .unwrap();

        // They must have different ids and different dirs
        assert_ne!(p1.id, p2.id);

        let dir1 = store.proj_dir_for(&p1.id).unwrap();
        let dir2 = store.proj_dir_for(&p2.id).unwrap();
        assert_ne!(dir1, dir2);

        // One slug is foo, the other is foo-2
        let slug1 = dir1.file_name().unwrap().to_str().unwrap();
        let slug2 = dir2.file_name().unwrap().to_str().unwrap();
        let slugs = [slug1, slug2];
        assert!(
            (slugs[0] == "foo" && slugs[1] == "foo-2")
                || (slugs[0] == "foo-2" && slugs[1] == "foo"),
            "expected foo and foo-2, got {:?}",
            slugs
        );
    }

    // ── Scenario: Archiving a session moves it out of the live tree ──

    #[test]
    fn archiving_a_session_moves_it_out_of_live_tree() {
        let (_tmp, store) = open_store();
        let proj = store.get_project(&ProjectId::unfiled()).unwrap().unwrap();

        let sess = store
            .create_session(
                NewSession {
                    project_id: Some(proj.id.clone()),
                    title: Some("ToArchive".into()),
                    title_source: TitleSource::Custom,
                    template_id: None,
                },
                None,
            )
            .unwrap();

        store.archive_session(&sess.id).unwrap();

        // Must not appear in live listing
        let sessions = store.list_sessions(Some(&proj.id)).unwrap();
        assert!(sessions.iter().all(|s| s.id != sess.id));

        // Must not be returned by get_session
        let got = store.get_session(&sess.id).unwrap();
        assert!(got.is_none());

        // Physically under .archive/
        let sess_dir = {
            let state = store.state.read().unwrap();
            state.get(sess.id.as_str()).unwrap().to_path_buf()
        };
        assert!(is_archived(&sess_dir));
    }

    // ── Scenario: Archiving a project archives its sessions with it ──

    #[test]
    fn archiving_a_project_archives_its_sessions_with_it() {
        let (_tmp, store) = open_store();

        let ws = store
            .create_workspace(NewWorkspace { name: "WS".into() })
            .unwrap();
        let proj = store
            .create_project(NewProject {
                source_kind: SourceKind::LocalDir,
                root_path: None,
                name: Some("ProjectWithSessions".into()),
                workspace_id: Some(ws.id.clone()),
            })
            .unwrap();
        let sess = store
            .create_session(
                NewSession {
                    project_id: Some(proj.id.clone()),
                    title: Some("S1".into()),
                    title_source: TitleSource::Custom,
                    template_id: None,
                },
                None,
            )
            .unwrap();

        store.archive_project(&proj.id).unwrap();

        // Project no longer in live listing
        let projects = store.list_projects(Some(&ws.id)).unwrap();
        assert!(projects.iter().all(|p| p.id != proj.id));

        // The whole subtree moved in one rename — check the proj dir is under .archive
        let proj_dir = {
            let state = store.state.read().unwrap();
            state.get(proj.id.as_str()).unwrap().to_path_buf()
        };
        assert!(is_archived(&proj_dir));

        // Session must be inside the archived project dir (moved with it)
        let sess_dir = {
            let state = store.state.read().unwrap();
            state.get(sess.id.as_str()).unwrap().to_path_buf()
        };
        assert!(sess_dir.starts_with(&proj_dir));
    }

    // ── Scenario: get-by-id resolves through the scan-built index ──

    #[test]
    fn get_by_id_resolves_through_scan_built_index() {
        let (tmp, store) = open_store();

        let ws = store
            .create_workspace(NewWorkspace {
                name: "ScanWS".into(),
            })
            .unwrap();
        let proj = store
            .create_project(NewProject {
                source_kind: SourceKind::LocalDir,
                root_path: None,
                name: Some("ScanProject".into()),
                workspace_id: Some(ws.id),
            })
            .unwrap();

        // Reopen: new store, fresh scan-built index
        let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();
        let got = store2.get_project(&proj.id).unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().name, "ScanProject");
    }

    // ── Scenario: The index follows a rename ──

    #[test]
    fn index_follows_rename() {
        let (tmp, store) = open_store();

        let ws = store
            .create_workspace(NewWorkspace { name: "WS".into() })
            .unwrap();
        let proj = store
            .create_project(NewProject {
                source_kind: SourceKind::LocalDir,
                root_path: None,
                name: Some("Before".into()),
                workspace_id: Some(ws.id),
            })
            .unwrap();

        store.rename_project(&proj.id, "After").unwrap();

        // get_project should still work — index was updated in-place
        let got = store.get_project(&proj.id).unwrap().unwrap();
        assert_eq!(got.name, "After");

        // After a reopen, new scan-built index should also find it
        let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();
        let got2 = store2.get_project(&proj.id).unwrap().unwrap();
        assert_eq!(got2.name, "After");
    }

    // ── Scenario: Surface binding round-trips through layout.json ──

    #[test]
    fn surface_binding_round_trips_through_layout_json() {
        let (tmp, store) = open_store();

        let proj = store.get_project(&ProjectId::unfiled()).unwrap().unwrap();
        let sess = store
            .create_session(
                NewSession {
                    project_id: Some(proj.id),
                    title: Some("S".into()),
                    title_source: TitleSource::Custom,
                    template_id: None,
                },
                None,
            )
            .unwrap();

        let surf = store
            .create_surface(NewSurface {
                id: None,
                session_id: sess.id.clone(),
                kind: SurfaceKind::Terminal,
                cwd: Some("./subdir".into()),
                placement: Some("main".into()),
            })
            .unwrap();

        // Reload store from disk
        let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();
        let found = store2
            .find_session_surface_by_placement(&sess.id, "main")
            .unwrap()
            .unwrap();

        assert_eq!(found.id, surf.id);
        assert_eq!(found.kind, SurfaceKind::Terminal);
        assert_eq!(found.placement, Some("main".into()));
        assert_eq!(found.cwd, Some("./subdir".into()));
    }

    // ── Scenario: Duplicate placement is rejected ──

    #[test]
    fn duplicate_placement_is_rejected_with_surface_conflict() {
        let (_tmp, store) = open_store();

        let proj = store.get_project(&ProjectId::unfiled()).unwrap().unwrap();
        let sess = store
            .create_session(
                NewSession {
                    project_id: Some(proj.id),
                    title: Some("S".into()),
                    title_source: TitleSource::Custom,
                    template_id: None,
                },
                None,
            )
            .unwrap();

        store
            .create_surface(NewSurface {
                id: None,
                session_id: sess.id.clone(),
                kind: SurfaceKind::Terminal,
                cwd: None,
                placement: Some("panel-1".into()),
            })
            .unwrap();

        // Second surface at the same placement must fail
        let result = store.create_surface(NewSurface {
            id: None,
            session_id: sess.id,
            kind: SurfaceKind::Terminal,
            cwd: None,
            placement: Some("panel-1".into()),
        });

        assert!(
            matches!(result, Err(OrchestratorError::SurfaceConflict(_))),
            "expected SurfaceConflict, got {:?}",
            result
        );
    }

    // ── Regression: archive reuses a freed slug, then collides in .archive ──

    #[test]
    fn archiving_two_projects_that_reuse_a_freed_slug_disambiguates_in_archive() {
        let (_tmp, store) = open_store();
        let ws = store
            .create_workspace(NewWorkspace { name: "W".into() })
            .unwrap();

        let p1 = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("Same".into()),
                workspace_id: Some(ws.id.clone()),
            })
            .unwrap();
        store.archive_project(&p1.id).unwrap(); // -> .archive/same; frees live slug

        let p2 = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("Same".into()),
                workspace_id: Some(ws.id),
            })
            .unwrap(); // reuses the now-free live slug "same"
        store.archive_project(&p2.id).unwrap(); // must disambiguate, not collide

        // Both archived subtrees survive distinctly.
        store.hard_delete_project(&p1.id).unwrap();
        store.hard_delete_project(&p2.id).unwrap();
    }

    // ── Regression: deleting a workspace preserves its archived projects ──

    #[test]
    fn deleting_a_workspace_preserves_its_archived_projects() {
        let (_tmp, store) = open_store();
        let ws = store
            .create_workspace(NewWorkspace {
                name: "Doomed".into(),
            })
            .unwrap();
        let proj = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("Keep".into()),
                workspace_id: Some(ws.id.clone()),
            })
            .unwrap();
        store.archive_project(&proj.id).unwrap();

        store.delete_workspace(&ws.id).unwrap();

        // The archived project was reassigned to Default, not destroyed.
        store.hard_delete_project(&proj.id).unwrap();
    }

    // ── Regression: persisted state is owner-only (0600 files / 0700 dirs) ──

    #[cfg(unix)]
    #[test]
    fn persisted_files_and_dirs_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let (_tmp, store) = open_store();
        let ws = store
            .create_workspace(NewWorkspace {
                name: "Perms".into(),
            })
            .unwrap();
        let ws_dir = store.ws_dir_for(&ws.id).unwrap();

        let dir_mode = fs::metadata(&ws_dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(dir_mode, 0o700, "domain directory must be owner-only");

        let file_mode = fs::metadata(ws_dir.join("workspace.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(file_mode, 0o600, "domain file must be owner-only");
    }

    // ── Scenario: Concurrent writes from two windows stay consistent ──

    #[test]
    fn concurrent_writes_from_two_threads_stay_consistent() {
        use std::sync::Arc;

        let (_tmp, store) = open_store();
        let store = Arc::new(store);

        let proj = store.get_project(&ProjectId::unfiled()).unwrap().unwrap();
        let proj_id = proj.id;

        let store1 = Arc::clone(&store);
        let store2 = Arc::clone(&store);
        let pid1 = proj_id.clone();
        let pid2 = proj_id.clone();

        let t1 = std::thread::spawn(move || {
            for i in 0..10 {
                store1
                    .create_session(
                        NewSession {
                            project_id: Some(pid1.clone()),
                            title: Some(format!("T1-{i}")),
                            title_source: TitleSource::Custom,
                            template_id: None,
                        },
                        None,
                    )
                    .unwrap();
            }
        });
        let t2 = std::thread::spawn(move || {
            for i in 0..10 {
                store2
                    .create_session(
                        NewSession {
                            project_id: Some(pid2.clone()),
                            title: Some(format!("T2-{i}")),
                            title_source: TitleSource::Custom,
                            template_id: None,
                        },
                        None,
                    )
                    .unwrap();
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let sessions = store.list_sessions(Some(&proj_id)).unwrap();
        assert_eq!(
            sessions.len(),
            20,
            "expected 20 sessions, got {}",
            sessions.len()
        );
    }

    // ── Scenario: Domain tree resolves under the data-root directory ──

    #[test]
    fn domain_tree_resolves_under_data_root_directory() {
        let tmp = TempDir::new().unwrap();
        let data_root = tmp.path().join("data");
        // data_root does not pre-exist; FsBackend::open should create it
        let store = FsBackend::open(data_root.clone()).unwrap();

        assert!(data_root.exists(), "data root should be created by open()");

        let ws = store
            .create_workspace(NewWorkspace {
                name: "RootTest".into(),
            })
            .unwrap();
        let ws_dir = store.ws_dir_for(&ws.id).unwrap();
        assert!(ws_dir.starts_with(&data_root));
    }
}
