//! Session store.

use crate::entities::{NewSession, ProjectId, Session, SessionId};
use crate::error::Result;
use crate::store::backend::Backend;

/// Typed filter for `Sessions::list`. Listing always excludes archived sessions.
#[derive(Clone, Default)]
pub struct SessionFilter {
    /// Scope to a single project; `None` lists across all projects.
    pub project: Option<ProjectId>,
}

/// Domain store for sessions. Knows nothing about launch templates -- cross-aggregate
/// template resolution lives in the `create_session` coordinator.
#[derive(Clone)]
pub struct Sessions {
    backend: Backend,
}

impl Sessions {
    pub fn new(backend: Backend) -> Self {
        Self { backend }
    }

    /// Materialize a session from a draft and a pre-resolved launch spec.
    pub async fn create(&self, draft: NewSession, spec: Option<(u32, String)>) -> Result<Session> {
        self.backend.create_session(draft, spec).await
    }

    pub async fn rename(&self, id: SessionId, title: String) -> Result<()> {
        self.backend.rename_session(id, title).await
    }

    pub async fn list(&self, filter: &SessionFilter) -> Result<Vec<Session>> {
        self.backend.list_sessions(filter.project.clone()).await
    }

    pub async fn get(&self, id: SessionId) -> Result<Option<Session>> {
        self.backend.get_session(id).await
    }

    pub async fn archive(&self, id: SessionId) -> Result<()> {
        self.backend.archive_session(id).await
    }

    pub async fn hard_delete(&self, id: SessionId) -> Result<()> {
        self.backend.hard_delete_session(id).await
    }

    pub async fn reorder(&self, id: SessionId, sort_order: u32) -> Result<()> {
        self.backend.reorder_session(id, sort_order).await
    }

    pub async fn set_spec(
        &self,
        id: SessionId,
        spec_version: u32,
        spec_json: String,
    ) -> Result<()> {
        self.backend
            .set_session_spec(id, spec_version, spec_json)
            .await
    }

    pub async fn set_layout(&self, id: SessionId, layout_json: String) -> Result<()> {
        self.backend.set_session_layout(id, layout_json).await
    }

    pub async fn get_layout(&self, id: SessionId) -> Result<Option<String>> {
        self.backend.get_session_layout(id).await
    }
}
