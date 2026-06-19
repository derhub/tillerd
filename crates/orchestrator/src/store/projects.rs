//! Project store.

use crate::entities::{NewProject, Project, ProjectId, WorkspaceId};
use crate::error::Result;
use crate::store::backend::Backend;

/// Typed filter for `Projects::list`. Listing always excludes archived projects.
#[derive(Clone, Default)]
pub struct ProjectFilter {
    /// Scope to a single workspace; `None` lists across all workspaces.
    pub workspace: Option<WorkspaceId>,
}

/// Domain store for projects.
#[derive(Clone)]
pub struct Projects {
    backend: Backend,
}

impl Projects {
    pub fn new(backend: Backend) -> Self {
        Self { backend }
    }

    pub async fn get(&self, id: ProjectId) -> Result<Option<Project>> {
        self.backend.get_project(id).await
    }

    pub async fn create(&self, draft: NewProject) -> Result<Project> {
        self.backend.create_project(draft).await
    }

    pub async fn rename(&self, id: ProjectId, name: String) -> Result<()> {
        self.backend.rename_project(id, name).await
    }

    pub async fn list(&self, filter: &ProjectFilter) -> Result<Vec<Project>> {
        self.backend.list_projects(filter.workspace.clone()).await
    }

    pub async fn move_to(&self, project_id: ProjectId, workspace_id: WorkspaceId) -> Result<()> {
        self.backend.move_project(project_id, workspace_id).await
    }

    pub async fn archive(&self, id: ProjectId) -> Result<()> {
        self.backend.archive_project(id).await
    }

    pub async fn hard_delete(&self, id: ProjectId) -> Result<()> {
        self.backend.hard_delete_project(id).await
    }

    pub async fn reorder(&self, id: ProjectId, sort_order: u32) -> Result<()> {
        self.backend.reorder_project(id, sort_order).await
    }
}
