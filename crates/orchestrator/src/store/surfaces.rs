//! Surface store.

use crate::entities::{NewSurface, SessionId, Surface, SurfaceId};
use crate::error::Result;
use crate::store::backend::Backend;

/// Domain store for surfaces.
#[derive(Clone)]
pub struct Surfaces {
    backend: Backend,
}

impl Surfaces {
    pub fn new(backend: Backend) -> Self {
        Self { backend }
    }

    pub async fn create(&self, draft: NewSurface) -> Result<Surface> {
        self.backend.create_surface(draft).await
    }

    pub async fn get(&self, id: SurfaceId) -> Result<Option<Surface>> {
        self.backend.get_surface(id).await
    }

    /// The session's live surface at `placement`, if any.
    pub async fn find_by_placement(
        &self,
        session_id: SessionId,
        placement: String,
    ) -> Result<Option<Surface>> {
        self.backend
            .find_session_surface_by_placement(session_id, placement)
            .await
    }

    pub async fn list_resumable(&self) -> Result<Vec<Surface>> {
        self.backend.list_resumable_surfaces().await
    }

    pub async fn update_status(&self, id: SurfaceId, status: String) -> Result<()> {
        self.backend.update_surface_status(id, status).await
    }

    pub async fn soft_delete(&self, id: SurfaceId) -> Result<()> {
        self.backend.soft_delete_surface(id).await
    }

    pub async fn add_to_session(&self, session_id: SessionId, surface_id: SurfaceId) -> Result<()> {
        self.backend
            .add_surface_to_session(session_id, surface_id)
            .await
    }

    pub async fn remove_from_session(
        &self,
        session_id: SessionId,
        surface_id: SurfaceId,
    ) -> Result<()> {
        self.backend
            .remove_surface_from_session(session_id, surface_id)
            .await
    }
}
