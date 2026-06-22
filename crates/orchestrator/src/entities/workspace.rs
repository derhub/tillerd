//! Workspace aggregate: the top of the tree. Strict containment — every project
//! belongs to exactly one workspace.

use serde::{Deserialize, Serialize};

use crate::shared::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Whether the workspace is active or archived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkspaceStatus {
    #[default]
    Active,
    Archived,
}

impl WorkspaceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkspaceStatus::Active => "active",
            WorkspaceStatus::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub sort_order: u32,
    pub pinned: bool,
    pub status: WorkspaceStatus,
}

impl Workspace {
    /// Rename the workspace. Trims whitespace.
    pub fn rename(&mut self, name: &str) {
        self.name = name.trim().to_owned();
    }

    /// Guard: reject mutation on the Default workspace.
    pub fn guard_not_default(&self) -> Result<()> {
        if self.id.is_default() {
            Err(Error::WorkspaceIsDefault)
        } else {
            Ok(())
        }
    }

    /// Guard: workspace must be Active to archive.
    pub fn guard_active(&self) -> Result<()> {
        if self.status == WorkspaceStatus::Archived {
            Err(Error::WorkspaceAlreadyArchived)
        } else {
            Ok(())
        }
    }

    /// Guard: workspace must be Archived to restore.
    pub fn guard_archived(&self) -> Result<()> {
        if self.status != WorkspaceStatus::Archived {
            Err(Error::WorkspaceNotArchived)
        } else {
            Ok(())
        }
    }
}

/// Parameters for creating a new workspace.
#[derive(Debug, Clone)]
pub struct NewWorkspace {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active_workspace(id: &str) -> Workspace {
        Workspace {
            id: WorkspaceId::new(id),
            name: "Test".to_owned(),
            sort_order: 0,
            pinned: false,
            status: WorkspaceStatus::Active,
        }
    }

    fn default_workspace() -> Workspace {
        Workspace {
            id: WorkspaceId::default_id(),
            name: "Default".to_owned(),
            sort_order: 0,
            pinned: false,
            status: WorkspaceStatus::Active,
        }
    }

    #[test]
    fn rename_trims_whitespace() {
        let mut ws = active_workspace("ws-1");
        ws.rename("  trimmed  ");
        assert_eq!(ws.name, "trimmed");
    }

    #[test]
    fn guard_not_default_allows_non_default() {
        let ws = active_workspace("ws-1");
        assert!(ws.guard_not_default().is_ok());
    }

    #[test]
    fn guard_not_default_rejects_default_workspace() {
        let ws = default_workspace();
        assert!(ws.guard_not_default().is_err());
    }

    #[test]
    fn guard_active_allows_active_workspace() {
        let ws = active_workspace("ws-1");
        assert!(ws.guard_active().is_ok());
    }

    #[test]
    fn guard_active_rejects_already_archived_workspace() {
        let ws = Workspace {
            status: WorkspaceStatus::Archived,
            ..active_workspace("ws-1")
        };
        assert!(ws.guard_active().is_err());
    }

    #[test]
    fn guard_archived_allows_archived_workspace() {
        let ws = Workspace {
            status: WorkspaceStatus::Archived,
            ..active_workspace("ws-1")
        };
        assert!(ws.guard_archived().is_ok());
    }

    #[test]
    fn guard_archived_rejects_active_workspace() {
        let ws = active_workspace("ws-1");
        assert!(ws.guard_archived().is_err());
    }
}
