//! Workspace aggregate: the top of the tree. Strict containment — every project
//! belongs to exactly one workspace.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    /// Fixed well-known id of the built-in Default workspace (mirrors `ProjectId::UNFILED`).
    pub const DEFAULT: &'static str = "00000000-0000-0000-0000-000000000001";

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn default_id() -> Self {
        Self(Self::DEFAULT.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_default(&self) -> bool {
        self.0 == Self::DEFAULT
    }
}

/// A named group of projects (the top of the tree). Strict containment: every project
/// belongs to exactly one workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
}

/// Parameters for creating a new workspace.
#[derive(Debug, Clone)]
pub struct NewWorkspace {
    pub name: String,
}
