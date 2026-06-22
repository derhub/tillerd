use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::SurfaceId;
use crate::infra::SurfaceRepo;
use crate::shared::errors::Result;
use crate::shared::message::Command;

use super::common::require_surface;

// -- CloseSurface (delete record) ------------------------------------------------

/// Remove a surface from a session: kill its runtime proxy and delete the record.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseSurface {
    pub id: String,
}

impl Command<Ctx> for CloseSurface {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = SurfaceId::from_string(&self.id);
        require_surface(cx, &id).await?;
        cx.runtime().close(&id).await?;
        SurfaceRepo::delete(cx.db(), &id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::get_surface_by_id::GetSurfaceById;
    use crate::app::surface::test_util::{harness, one_surface, seed_session, spawn};

    // Close deletes the record and kills the runtime proxy
    #[tokio::test]
    async fn close_deletes_the_record_and_kills_the_proxy() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-close").await;
        h.bus.execute(spawn(&session)).await.unwrap();
        let surface = one_surface(&h, &session).await;

        h.bus
            .execute(CloseSurface {
                id: surface.id.clone(),
            })
            .await
            .unwrap();

        let gone = h
            .bus
            .query(GetSurfaceById {
                id: surface.id.clone(),
            })
            .await
            .unwrap();
        assert!(gone.is_none());
        assert!(!h.runtime.is_running(&SurfaceId::from_string(&surface.id)));
    }
}
