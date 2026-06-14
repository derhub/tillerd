pub mod memory;
pub mod schema;
pub mod sqlite;

pub use schema::current_version as current_schema_version;
pub use sqlite::SqliteStore;

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    pub fn mint() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SurfaceId(String);

impl SurfaceId {
    pub fn mint() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectId(String);

impl ProjectId {
    pub const UNFILED: &'static str = "00000000-0000-0000-0000-000000000000";

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn unfiled() -> Self {
        Self(Self::UNFILED.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_unfiled(&self) -> bool {
        self.0 == Self::UNFILED
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Blank,
    LocalDir,
    GitRepo,
    GitWorktree,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Blank => "blank",
            SourceKind::LocalDir => "local_dir",
            SourceKind::GitRepo => "git_repo",
            SourceKind::GitWorktree => "git_worktree",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    Terminal,
    Diff,
}

impl SurfaceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SurfaceKind::Terminal => "terminal",
            SurfaceKind::Diff => "diff",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub source_kind: SourceKind,
    pub root_path: Option<String>,
}

/// How a session's display title is derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TitleSource {
    /// Populated when the agent reports a title on completion.
    #[default]
    AgentTitle,
    /// Set to the git branch of the session root at creation time.
    Branch,
    /// Concatenation of branch (at creation) and agent title (when available).
    Both,
    /// Caller-supplied verbatim title.
    Custom,
}

impl TitleSource {
    pub fn as_str(self) -> &'static str {
        match self {
            TitleSource::AgentTitle => "agent-title",
            TitleSource::Branch => "branch",
            TitleSource::Both => "both",
            TitleSource::Custom => "custom",
        }
    }
}

/// Parameters for creating a new project.
#[derive(Debug, Clone)]
pub struct NewProject {
    pub source_kind: SourceKind,
    pub root_path: Option<String>,
    /// Explicit name; overrides inference when supplied.
    pub name: Option<String>,
}

/// Parameters for creating a new session.
#[derive(Debug, Clone, Default)]
pub struct NewSession {
    pub project_id: Option<ProjectId>,
    pub title_source: TitleSource,
    /// Required when `title_source == Custom`; used as branch/agent-title for other strategies.
    pub title: Option<String>,
    /// When supplied, the session's spec blob and version are copied atomically from this template.
    pub template_id: Option<LaunchTemplateId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: SessionId,
    pub project_id: ProjectId,
    pub title: String,
    pub title_source: TitleSource,
    pub created_at: String,
    pub spec_version: Option<u32>,
    pub spec_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewSurface {
    pub id: Option<SurfaceId>,
    pub session_id: SessionId,
    pub kind: SurfaceKind,
    pub cwd: Option<String>,
    pub placement: Option<String>,
    pub worktree_id: Option<WorktreeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface {
    pub id: SurfaceId,
    pub session_id: SessionId,
    pub kind: SurfaceKind,
    pub cwd: Option<String>,
    pub last_status: Option<String>,
    pub placement: Option<String>,
    pub worktree_id: Option<WorktreeId>,
}

impl Surface {
    pub fn correlation_id(&self) -> &SurfaceId {
        &self.id
    }
}

// ── command library ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandId(String);

impl CommandId {
    pub fn mint() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandOrigin {
    Prebuilt,
    Custom,
}

impl CommandOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            CommandOrigin::Prebuilt => "prebuilt",
            CommandOrigin::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub id: CommandId,
    pub name: String,
    pub origin: CommandOrigin,
    pub cli: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct NewCommand {
    pub name: String,
    pub origin: CommandOrigin,
    pub cli: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
}

// ── worktree ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorktreeId(String);

impl WorktreeId {
    pub fn mint() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    pub id: WorktreeId,
    pub project_id: ProjectId,
    pub path: String,
    pub branch: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewWorktree {
    pub project_id: ProjectId,
    pub path: String,
    pub branch: Option<String>,
}

// ── launch template ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LaunchTemplateId(String);

impl LaunchTemplateId {
    pub fn mint() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchTemplate {
    pub id: LaunchTemplateId,
    pub project_id: ProjectId,
    pub spec_version: u32,
    pub spec_json: String,
}

#[derive(Debug, Clone)]
pub struct NewLaunchTemplate {
    pub project_id: ProjectId,
    pub spec_version: u32,
    pub spec_json: String,
}

// ── settings ──────────────────────────────────────────────────────────────────

/// Scope a setting is stored under: app-global, or bound to a specific project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingScope {
    Global,
    Project(ProjectId),
}

impl SettingScope {
    /// The `(scope, project_id)` column pair for the `setting` table. Global uses an
    /// empty `project_id` sentinel — never NULL — so the composite primary key
    /// `(scope, project_id, key)` stays unique and upsert works (SQLite treats NULLs
    /// as distinct, which would defeat both).
    pub fn columns(&self) -> (&'static str, &str) {
        match self {
            SettingScope::Global => ("global", ""),
            SettingScope::Project(id) => ("project", id.as_str()),
        }
    }
}

/// A stored setting: its key and JSON-encoded value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingEntry {
    pub key: String,
    pub value_json: String,
}

/// A durably-stored user-facing notification (ADR-0031). `ts` is event time in epoch
/// milliseconds; `actions_json` is a JSON-encoded action list when present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationRecord {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub title: Option<String>,
    pub message: String,
    pub detail: Option<String>,
    pub ts: i64,
    pub session_id: Option<String>,
    pub surface_id: Option<String>,
    pub actions_json: Option<String>,
}

pub trait Store: Send + Sync {
    fn schema_version(&self) -> Result<u32>;

    // ── project ──────────────────────────────────────────────────────────

    fn get_project(&self, id: &ProjectId) -> Result<Option<Project>>;

    /// Create a project; infers name from source when `draft.name` is `None`.
    fn create_project(&self, draft: NewProject) -> Result<Project>;

    /// Rename a project. Returns `ProjectNotFound` for unknown id,
    /// `ProjectIsUnfiled` for the built-in Unfiled project.
    fn rename_project(&self, id: &ProjectId, name: &str) -> Result<()>;

    /// Return non-archived projects ordered by `created_at` descending.
    fn list_projects(&self) -> Result<Vec<Project>>;

    /// Soft-delete a project and cascade to its sessions and surfaces atomically.
    /// Returns `ProjectIsUnfiled` for the built-in project, `ProjectNotFound` if missing.
    fn archive_project(&self, id: &ProjectId) -> Result<()>;

    /// Permanently remove an already-archived project and all descendant rows.
    /// Returns `ProjectNotArchived` if the project is not archived.
    fn hard_delete_project(&self, id: &ProjectId) -> Result<()>;

    /// Reorder a project to a new sort position.
    /// Returns `ProjectNotFound` if the project does not exist.
    fn reorder_project(&self, id: &ProjectId, sort_order: u32) -> Result<()>;

    // ── session ───────────────────────────────────────────────────────────

    fn create_session(&self, draft: NewSession) -> Result<Session>;

    /// Rename a session and set `title_source` to `Custom`.
    /// Returns `SessionNotFound` for unknown id.
    fn rename_session(&self, id: &SessionId, title: &str) -> Result<()>;

    /// Return non-archived sessions. Pass `Some(project_id)` to filter by project.
    fn list_sessions(&self, project_id: Option<&ProjectId>) -> Result<Vec<Session>>;

    /// Get a single session by id (non-archived).
    fn get_session(&self, id: &SessionId) -> Result<Option<Session>>;

    /// Soft-delete a session and cascade to its surfaces atomically.
    fn archive_session(&self, id: &SessionId) -> Result<()>;

    /// Permanently remove an already-archived session and its surface rows.
    fn hard_delete_session(&self, id: &SessionId) -> Result<()>;

    /// Reorder a session to a new sort position.
    /// Returns `SessionNotFound` if the session does not exist.
    fn reorder_session(&self, id: &SessionId, sort_order: u32) -> Result<()>;

    // ── surface ───────────────────────────────────────────────────────────

    fn create_surface(&self, draft: NewSurface) -> Result<Surface>;

    fn get_surface(&self, id: &SurfaceId) -> Result<Option<Surface>>;

    /// The session's live surface at `placement`, if any (the one to re-attach to on revisit).
    fn find_session_surface_by_placement(
        &self,
        session_id: &SessionId,
        placement: &str,
    ) -> Result<Option<Surface>>;

    fn list_resumable_surfaces(&self) -> Result<Vec<Surface>>;

    fn update_surface_status(&self, id: &SurfaceId, status: &str) -> Result<()>;

    fn soft_delete_surface(&self, id: &SurfaceId) -> Result<()>;

    /// Associate a surface with a session.
    /// Returns `SurfaceConflict` if the surface already belongs to a different session.
    fn add_surface_to_session(&self, session_id: &SessionId, surface_id: &SurfaceId) -> Result<()>;

    /// Soft-delete a surface without terminating its PTY.
    fn remove_surface_from_session(
        &self,
        session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()>;

    // ── layout ────────────────────────────────────────────────────────────

    /// Replace a session's launch spec blob and version (used when spawn/close diverge the spec).
    /// Returns `SessionNotFound` if the session does not exist.
    fn set_session_spec(&self, id: &SessionId, spec_version: u32, spec_json: &str) -> Result<()>;

    /// Persist the layout JSON blob for a session.
    /// Returns `SessionNotFound` if the session does not exist.
    fn set_session_layout(&self, id: &SessionId, layout_json: &str) -> Result<()>;

    /// Return the stored layout JSON blob, or `None` if not yet set.
    fn get_session_layout(&self, id: &SessionId) -> Result<Option<String>>;

    // ── command library ───────────────────────────────────────────────────

    /// Return all non-deleted commands.
    fn list_commands(&self) -> Result<Vec<Command>>;

    /// Return a single command by id, or `None` if not found.
    fn get_command(&self, id: &str) -> Result<Option<Command>>;

    /// Persist a new command and return it.
    fn create_command(&self, draft: NewCommand) -> Result<Command>;

    /// Soft-delete a command by id.
    fn delete_command(&self, id: &str) -> Result<()>;

    /// Insert prebuilt seed entries if they are absent (idempotent).
    fn seed_commands(&self) -> Result<()>;

    // ── worktree ──────────────────────────────────────────────────────────

    /// Persist a new worktree row and return it.
    fn create_worktree(&self, draft: NewWorktree) -> Result<Worktree>;

    /// Return non-archived worktrees for the given project.
    fn list_worktrees(&self, project_id: &ProjectId) -> Result<Vec<Worktree>>;

    /// Soft-delete a worktree row.
    fn archive_worktree(&self, id: &WorktreeId) -> Result<()>;

    // ── launch template ───────────────────────────────────────────────────

    /// Persist a new launch template.
    fn create_launch_template(&self, draft: NewLaunchTemplate) -> Result<LaunchTemplate>;

    /// Return a template by id.
    fn get_launch_template(&self, id: &LaunchTemplateId) -> Result<Option<LaunchTemplate>>;

    /// Update the spec blob and version for an existing template.
    fn set_launch_template_spec(
        &self,
        id: &LaunchTemplateId,
        spec_version: u32,
        spec_json: &str,
    ) -> Result<()>;

    // ── settings ──────────────────────────────────────────────────────────

    /// Read a setting's JSON value for an exact scope, or `None` if unset.
    fn get_setting(&self, scope: &SettingScope, key: &str) -> Result<Option<String>>;

    /// Insert or replace a setting's JSON value for a scope.
    fn set_setting(&self, scope: &SettingScope, key: &str, value_json: &str) -> Result<()>;

    /// All settings stored under a scope.
    fn list_settings(&self, scope: &SettingScope) -> Result<Vec<SettingEntry>>;

    /// Resolve a key for a project: the project-scoped value if present, else the
    /// global value, else `None`.
    fn resolve_setting(&self, project_id: &ProjectId, key: &str) -> Result<Option<String>> {
        if let Some(v) = self.get_setting(&SettingScope::Project(project_id.clone()), key)? {
            return Ok(Some(v));
        }
        self.get_setting(&SettingScope::Global, key)
    }

    // ── notifications (ADR-0031) ──────────────────────────────────────────

    /// Append a notification to the durable history.
    fn insert_notification(&self, rec: &NotificationRecord) -> Result<()>;

    /// The most recent `limit` notifications, newest first.
    fn list_notifications(&self, limit: u32) -> Result<Vec<NotificationRecord>>;

    /// Retain only the most recent `keep` notifications, discarding older ones.
    fn prune_notifications(&self, keep: u32) -> Result<()>;
}
