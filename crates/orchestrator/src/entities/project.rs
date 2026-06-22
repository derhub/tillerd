//! Project aggregate: a source root grouped under a workspace.

use serde::{Deserialize, Serialize};

use super::workspace::WorkspaceId;
use crate::shared::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
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

/// Whether the project is active or archived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum ProjectStatus {
    #[default]
    Active,
    Archived,
}

impl ProjectStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectStatus::Active => "active",
            ProjectStatus::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub source_kind: SourceKind,
    pub root_path: Option<String>,
    /// Owning workspace; never null at rest after the workspace migration.
    pub workspace_id: WorkspaceId,
    pub sort_order: u32,
    pub pinned: bool,
    pub status: ProjectStatus,
}

impl Project {
    /// Rename the project. Trims whitespace.
    pub fn rename(&mut self, name: &str) {
        self.name = name.trim().to_owned();
    }

    /// Guard: reject mutation on the Unfiled project.
    pub fn guard_not_unfiled(&self) -> Result<()> {
        if self.id.is_unfiled() {
            Err(Error::ProjectIsUnfiled)
        } else {
            Ok(())
        }
    }

    /// Guard: project must be Archived before hard-delete.
    pub fn guard_archived(&self) -> Result<()> {
        if self.status != ProjectStatus::Archived {
            Err(Error::ProjectNotArchived)
        } else {
            Ok(())
        }
    }

    /// Guard: project must be Active to archive.
    pub fn guard_active(&self) -> Result<()> {
        if self.status == ProjectStatus::Archived {
            Err(Error::ProjectAlreadyArchived)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active_project(id: &str) -> Project {
        Project {
            id: ProjectId::new(id),
            name: "Test".to_owned(),
            source_kind: SourceKind::Blank,
            root_path: None,
            workspace_id: WorkspaceId::new("ws-1"),
            sort_order: 0,
            pinned: false,
            status: ProjectStatus::Active,
        }
    }

    fn unfiled_project() -> Project {
        Project {
            id: ProjectId::unfiled(),
            name: "Unfiled".to_owned(),
            source_kind: SourceKind::Blank,
            root_path: None,
            workspace_id: WorkspaceId::default_id(),
            sort_order: 0,
            pinned: false,
            status: ProjectStatus::Active,
        }
    }

    #[test]
    fn rename_trims_whitespace() {
        let mut p = active_project("p-1");
        p.rename("  new name  ");
        assert_eq!(p.name, "new name");
    }

    #[test]
    fn guard_not_unfiled_allows_normal_project() {
        let p = active_project("p-1");
        assert!(p.guard_not_unfiled().is_ok());
    }

    #[test]
    fn guard_not_unfiled_rejects_unfiled_project() {
        let p = unfiled_project();
        assert!(p.guard_not_unfiled().is_err());
    }

    #[test]
    fn guard_archived_allows_archived_project() {
        let p = Project {
            status: ProjectStatus::Archived,
            ..active_project("p-1")
        };
        assert!(p.guard_archived().is_ok());
    }

    #[test]
    fn guard_archived_rejects_active_project() {
        let p = active_project("p-1");
        assert!(p.guard_archived().is_err());
    }

    #[test]
    fn guard_active_allows_active_project() {
        let p = active_project("p-1");
        assert!(p.guard_active().is_ok());
    }

    #[test]
    fn guard_active_rejects_already_archived_project() {
        let p = Project {
            status: ProjectStatus::Archived,
            ..active_project("p-1")
        };
        assert!(p.guard_active().is_err());
    }
}
