//! The durable product store, behind a repository [`Store`] trait.
//!
//! All SQL lives in [`sqlite`]; domain, boot, and supervision code depend only on
//! the trait and the row/domain types here (design: repository-trait seam). An
//! in-memory [`memory::InMemoryStore`] fake implements the same trait for tests,
//! including other crates' tests.
//!
//! The store realizes the ADR-0023 two-level id model: [`SessionId`] is the
//! product-layer container id and lives only in the store; [`SurfaceId`] is the
//! leaf id shared across backends (it equals the daemon PTY id, the gate session
//! id, and the correlation id). No backend-facing operation accepts a
//! `SessionId`, so the product session id can never leave the orchestrator.

pub mod memory;
pub mod schema;
pub mod sqlite;

pub use schema::current_version as current_schema_version;
pub use sqlite::SqliteStore;

use crate::error::Result;

/// Product-layer session (container) identifier. Exists only in the store and is
/// never exposed to a backend service (ADR-0023 two-level id model).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Mint a fresh random session id.
    pub fn mint() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Borrow the underlying identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Leaf-layer surface identifier — the only id shared across backends. It equals
/// the daemon PTY id, the gate session id, and the correlation id (ADR-0020/0023).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SurfaceId(String);

impl SurfaceId {
    /// Mint a fresh random surface id.
    pub fn mint() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Borrow the underlying identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Project identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectId(String);

impl ProjectId {
    /// The fixed identifier of the seeded "Unfiled" project, to which every
    /// session that names no project belongs (ADR-0023 seeds).
    pub const UNFILED: &'static str = "00000000-0000-0000-0000-000000000000";

    /// Wrap an existing identifier string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The fixed Unfiled project id.
    pub fn unfiled() -> Self {
        Self(Self::UNFILED.to_string())
    }

    /// Borrow the underlying identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// How a project's working tree is sourced (ADR-0023 `project.source_kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// No backing directory.
    Blank,
    /// A plain local directory.
    LocalDir,
    /// A git repository.
    GitRepo,
    /// A git worktree.
    GitWorktree,
}

impl SourceKind {
    /// The schema string for this kind.
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Blank => "blank",
            SourceKind::LocalDir => "local_dir",
            SourceKind::GitRepo => "git_repo",
            SourceKind::GitWorktree => "git_worktree",
        }
    }
}

/// The kind of surface a session holds (ADR-0023 `surface.kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    /// A terminal surface.
    Terminal,
    /// An agent surface.
    Agent,
    /// A diff surface.
    Diff,
}

impl SurfaceKind {
    /// The schema string for this kind.
    pub fn as_str(self) -> &'static str {
        match self {
            SurfaceKind::Terminal => "terminal",
            SurfaceKind::Agent => "agent",
            SurfaceKind::Diff => "diff",
        }
    }
}

/// A project row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// The project identifier.
    pub id: ProjectId,
    /// Display name.
    pub name: String,
    /// How the project's tree is sourced.
    pub source_kind: SourceKind,
    /// The project root, if any.
    pub root_path: Option<String>,
}

/// Fields for creating a session. A `None` `project` resolves to the seeded
/// Unfiled project so `session.project_id` is never null.
#[derive(Debug, Clone, Default)]
pub struct NewSession {
    /// The owning project, or `None` to resolve to Unfiled.
    pub project: Option<ProjectId>,
    /// Optional title.
    pub title: Option<String>,
}

/// A session row. Its [`SessionId`] is product-only and is never handed to a
/// backend service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// Product-layer session id (internal to the orchestrator).
    pub id: SessionId,
    /// The owning project (never null; resolves to Unfiled when unspecified).
    pub project_id: ProjectId,
    /// Optional title.
    pub title: Option<String>,
}

/// Fields for creating a surface under a session.
#[derive(Debug, Clone)]
pub struct NewSurface {
    /// The owning session.
    pub session_id: SessionId,
    /// The surface kind.
    pub kind: SurfaceKind,
    /// Optional working directory.
    pub cwd: Option<String>,
}

/// A surface row. Its [`SurfaceId`] is the shared kernel reused across backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface {
    /// Leaf-layer surface id — shared with the daemon and gate.
    pub id: SurfaceId,
    /// The owning session.
    pub session_id: SessionId,
    /// The surface kind.
    pub kind: SurfaceKind,
    /// Optional working directory.
    pub cwd: Option<String>,
}

impl Surface {
    /// The identifier handed to backends for this surface — its surface id, and
    /// nothing else. This is the only id that crosses the orchestrator boundary
    /// (ADR-0023). There is deliberately no accessor that yields the session id
    /// to a backend.
    pub fn correlation_id(&self) -> &SurfaceId {
        &self.id
    }
}

/// The durable product store, as a repository over row/domain types.
///
/// Implementors confine all storage details (SQL, files) behind this trait so
/// domain, boot, and supervision depend only on it. The store is owned by the
/// orchestrator and accessed only in Rust; the renderer reaches it through the
/// orchestrator API, never directly.
pub trait Store: Send + Sync {
    /// The store's current schema version, as recorded in its `meta` record.
    fn schema_version(&self) -> Result<u32>;

    /// Fetch a project by id, or `None` if absent or soft-deleted.
    fn get_project(&self, id: &ProjectId) -> Result<Option<Project>>;

    /// Create a session, resolving an unspecified project to Unfiled and minting
    /// a product-only [`SessionId`].
    fn create_session(&self, draft: NewSession) -> Result<Session>;

    /// Create a surface under a session, minting a shared [`SurfaceId`].
    fn create_surface(&self, draft: NewSurface) -> Result<Surface>;
}
