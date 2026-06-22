use crate::context::Ctx;
use crate::entities::{SurfaceId, SurfaceStatus};
use crate::infra::SurfaceRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::Result;

use super::common::require_surface;

// ── StopSurface (keep record, resumable) ────────────────────────────────────────

/// Kill the process inside a surface; the record is kept so it can resume later.
#[derive(Debug, Clone)]
pub struct StopSurface {
    pub id: SurfaceId,
}

impl Command<Ctx> for StopSurface {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        require_surface(cx, &self.id).await?;
        cx.runtime().stop(&self.id).await?;
        SurfaceRepo::update_status(cx.db(), &self.id, SurfaceStatus::Idle).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::get_surface_by_id::GetSurfaceById;
    use crate::app::surface::test_util::{harness, one_surface, seed_session, spawn};
    use crate::shared::Error;

    // Scenario: A command mutates and returns nothing — Stop keeps the record idle
    #[tokio::test]
    async fn stop_marks_idle_and_keeps_the_record() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-stop").await;
        h.bus.execute(spawn(&session)).await.unwrap();
        let surface = one_surface(&h, &session).await;

        h.bus
            .execute(StopSurface {
                id: surface.id.clone(),
            })
            .await
            .unwrap();

        let after = h
            .bus
            .query(GetSurfaceById {
                id: surface.id.clone(),
            })
            .await
            .unwrap()
            .expect("record kept");
        assert_eq!(after.status, SurfaceStatus::Idle);
        assert!(!h.runtime.is_running(&surface.id));
    }

    #[tokio::test]
    async fn stop_rejects_an_unknown_surface() {
        let h = harness().await;
        let result = h
            .bus
            .execute(StopSurface {
                id: SurfaceId::from_string("no-such"),
            })
            .await;
        assert!(matches!(result, Err(Error::SurfaceNotFound(_))));
    }
}
