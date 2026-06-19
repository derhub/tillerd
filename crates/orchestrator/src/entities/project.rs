//! Project aggregate: a source root grouped under a workspace.

use super::workspace::WorkspaceId;

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
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Blank => "blank",
            SourceKind::LocalDir => "local_dir",
            SourceKind::GitRepo => "git_repo",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub source_kind: SourceKind,
    pub root_path: Option<String>,
    /// Owning workspace; never null at rest after the workspace migration.
    pub workspace_id: WorkspaceId,
}

/// Parameters for creating a new project.
#[derive(Debug, Clone)]
pub struct NewProject {
    pub source_kind: SourceKind,
    pub root_path: Option<String>,
    /// Explicit name; overrides inference when supplied.
    pub name: Option<String>,
    /// Owning workspace; defaults to the Default workspace when `None`.
    pub workspace_id: Option<WorkspaceId>,
}
