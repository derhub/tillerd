use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::{SurfaceId, SurfaceStatus};
use crate::shared::errors::Result;
use crate::shared::message::Command;

use super::common::require_surface;

/// Kill the process inside a surface; the record is kept so it can resume later.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopSurface {
    pub id: String,
}

impl Command<Ctx> for StopSurface {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = SurfaceId::from_string(&self.id);
        let surface = require_surface(cx, &id).await?;
        let workspace_id =
            super::status_events::workspace_id_for_session(cx, &surface.session_id).await?;
        cx.runtime().stop(&id).await?;
        super::status_events::update_status_and_emit(cx, &id, &workspace_id, SurfaceStatus::Idle)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::get_surface_by_id::GetSurfaceById;
    use crate::app::surface::test_util::{harness, one_surface, seed_session, spawn};
    use crate::shared::Error;

    // Scenario: A command mutates and returns nothing -- Stop keeps the record idle
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
        assert_eq!(after.status, "idle");
        assert!(!h.runtime.is_running(&SurfaceId::from_string(&surface.id)));
    }

    #[tokio::test]
    async fn stop_rejects_an_unknown_surface() {
        let h = harness().await;
        let result = h
            .bus
            .execute(StopSurface {
                id: "no-such".to_owned(),
            })
            .await;
        assert!(matches!(result, Err(Error::SurfaceNotFound(_))));
    }
}
