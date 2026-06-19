//! In-memory domain + operational store: `infra/memory/`.
//!
//! `MemoryBackend` holds every entity in a single `Mutex<Inner>` of `HashMap`s,
//! mirroring the fs/sqlite backends for tests and ephemeral runs. One entity file
//! per store (`project`/`workspace`/`session`/`surface` + `command`/`launch_template`/
//! `setting`/`notification`), each an `impl MemoryBackend` over `use super::*`.

pub(crate) use std::cmp::Reverse;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Mutex;

pub(crate) use super::schema::current_version;
pub(crate) use crate::entities::{
    Command, CommandId, CommandOrigin, LaunchTemplate, LaunchTemplateId, NewCommand,
    NewLaunchTemplate, NewProject, NewSession, NewSurface, NewWorkspace, NotificationRecord,
    Project, ProjectId, Session, SessionId, SettingEntry, SettingScope, SourceKind, Surface,
    SurfaceId, TitleSource, Workspace, WorkspaceId,
};
pub(crate) use crate::error::{OrchestratorError, Result};

mod command;
mod launch_template;
mod notification;
mod project;
mod session;
mod setting;
mod surface;
mod workspace;

pub struct MemoryBackend {
    inner: Mutex<Inner>,
}

struct Inner {
    version: u32,
    workspaces: HashMap<String, WorkspaceRecord>,
    projects: HashMap<String, ProjectRecord>,
    sessions: HashMap<String, SessionRecord>,
    surfaces: HashMap<String, SurfaceRecord>,
    commands: HashMap<String, CommandRecord>,
    launch_templates: HashMap<String, LaunchTemplate>,
    /// Keyed by (scope, project_id, key) -> value_json, mirroring the sqlite primary key.
    settings: HashMap<(String, String, String), String>,
    /// Insertion-ordered notification history (oldest first); mirrors the sqlite rowid order.
    notifications: Vec<NotificationRecord>,
}

#[derive(Clone)]
struct CommandRecord {
    command: Command,
    deleted: bool,
}

#[derive(Clone)]
struct ProjectRecord {
    project: Project,
    deleted: bool,
    created_seq: u64,
}

#[derive(Clone)]
struct WorkspaceRecord {
    workspace: Workspace,
    sort_order: u32,
    created_seq: u64,
}

#[derive(Clone)]
struct SessionRecord {
    session: Session,
    deleted: bool,
    layout_json: Option<String>,
}

#[derive(Clone)]
struct SurfaceRecord {
    surface: Surface,
    deleted: bool,
}

impl MemoryBackend {
    pub fn new() -> Self {
        let mut workspaces = HashMap::new();
        workspaces.insert(
            WorkspaceId::DEFAULT.to_string(),
            WorkspaceRecord {
                workspace: Workspace {
                    id: WorkspaceId::default_id(),
                    name: "Default".to_string(),
                },
                sort_order: 0,
                created_seq: 0,
            },
        );
        let mut projects = HashMap::new();
        projects.insert(
            ProjectId::UNFILED.to_string(),
            ProjectRecord {
                project: Project {
                    id: ProjectId::unfiled(),
                    name: "Unfiled".to_string(),
                    source_kind: SourceKind::Blank,
                    root_path: None,
                    workspace_id: WorkspaceId::default_id(),
                },
                deleted: false,
                created_seq: 0,
            },
        );
        let store = Self {
            inner: Mutex::new(Inner {
                version: current_version(),
                workspaces,
                projects,
                sessions: HashMap::new(),
                surfaces: HashMap::new(),
                commands: HashMap::new(),
                launch_templates: HashMap::new(),
                settings: HashMap::new(),
                notifications: Vec::new(),
            }),
        };
        // Seed prebuilt commands on creation (idempotent).
        let _ = store.seed_commands();
        store
    }

    /// Materialize a session from a draft and a pre-resolved launch spec, with no
    /// template lookup. The backend primitive; template->spec resolution lives in the
    /// `create_session` coordinator.
    pub(crate) fn create_session_inner(
        &self,
        draft: NewSession,
        spec: Option<(u32, String)>,
    ) -> Result<Session> {
        let (spec_version, spec_json) = match spec {
            Some((version, json)) => (Some(version), Some(json)),
            None => (None, None),
        };
        let session = Session {
            id: SessionId::mint(),
            project_id: draft.project_id.unwrap_or_else(ProjectId::unfiled),
            title: draft.title.unwrap_or_default(),
            title_source: draft.title_source,
            created_at: chrono_now(),
            spec_version,
            spec_json,
        };
        self.inner.lock().unwrap().sessions.insert(
            session.id.as_str().to_string(),
            SessionRecord {
                session: session.clone(),
                deleted: false,
                layout_json: None,
            },
        );
        Ok(session)
    }

    pub(crate) fn schema_version(&self) -> Result<u32> {
        Ok(self.inner.lock().unwrap().version)
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

fn prebuilt_commands_mem() -> Vec<Command> {
    vec![Command {
        id: CommandId::from_string("00000000-0000-0000-0000-000000000101"),
        name: "login-shell".to_string(),
        origin: CommandOrigin::Prebuilt,
        cli: "/bin/bash".to_string(),
        args: vec!["-l".to_string()],
        env: Default::default(),
    }]
}

fn infer_project_name(source: SourceKind, root_path: Option<&str>) -> Option<String> {
    match source {
        SourceKind::Blank => None,
        SourceKind::LocalDir | SourceKind::GitRepo => root_path.and_then(|p| {
            std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        }),
    }
}

fn chrono_now() -> String {
    // Minimal ISO-like timestamp without pulling in chrono
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}

#[cfg(test)]
mod tests;
