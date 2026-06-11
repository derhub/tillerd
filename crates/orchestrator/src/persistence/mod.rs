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
    Agent,
    Diff,
}

impl SurfaceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SurfaceKind::Terminal => "terminal",
            SurfaceKind::Agent => "agent",
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

#[derive(Debug, Clone, Default)]
pub struct NewSession {
    pub project: Option<ProjectId>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: SessionId,
    pub project_id: ProjectId,
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewSurface {
    pub session_id: SessionId,
    pub kind: SurfaceKind,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface {
    pub id: SurfaceId,
    pub session_id: SessionId,
    pub kind: SurfaceKind,
    pub cwd: Option<String>,
    /// The last known status of this surface (e.g. `"running"`, `"exited"`).
    /// `None` until explicitly set via [`Store::update_surface_status`].
    pub last_status: Option<String>,
}

impl Surface {
    pub fn correlation_id(&self) -> &SurfaceId {
        &self.id
    }
}

pub trait Store: Send + Sync {
    fn schema_version(&self) -> Result<u32>;

    fn get_project(&self, id: &ProjectId) -> Result<Option<Project>>;

    fn create_session(&self, draft: NewSession) -> Result<Session>;

    fn create_surface(&self, draft: NewSurface) -> Result<Surface>;

    /// Fetch a single non-deleted surface by id.
    /// Returns `None` when the surface does not exist or has been soft-deleted.
    fn get_surface(&self, id: &SurfaceId) -> Result<Option<Surface>>;

    /// List all surfaces that have not been soft-deleted.
    fn list_resumable_surfaces(&self) -> Result<Vec<Surface>>;

    /// Set `last_status` on the identified surface.
    fn update_surface_status(&self, id: &SurfaceId, status: &str) -> Result<()>;

    /// Soft-delete the identified surface by recording a deletion timestamp.
    fn soft_delete_surface(&self, id: &SurfaceId) -> Result<()>;
}
