//! Workspace store.

use crate::entities::{NewWorkspace, Workspace, WorkspaceId};
use crate::error::Result;
use crate::store::backend::Backend;

/// Domain store for workspaces.
#[derive(Clone)]
pub struct Workspaces {
    backend: Backend,
}

impl Workspaces {
    pub fn new(backend: Backend) -> Self {
        Self { backend }
    }

    pub async fn create(&self, draft: NewWorkspace) -> Result<Workspace> {
        self.backend.create_workspace(draft).await
    }

    pub async fn rename(&self, id: WorkspaceId, name: String) -> Result<()> {
        self.backend.rename_workspace(id, name).await
    }

    pub async fn list(&self) -> Result<Vec<Workspace>> {
        self.backend.list_workspaces().await
    }

    pub async fn reorder(&self, id: WorkspaceId, sort_order: u32) -> Result<()> {
        self.backend.reorder_workspace(id, sort_order).await
    }

    pub async fn delete(&self, id: WorkspaceId) -> Result<()> {
        self.backend.delete_workspace(id).await
    }
}
