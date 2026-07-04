use serde::Deserialize;

use crate::app::surface::SurfaceView;
use crate::context::Ctx;
use crate::shared::errors::Result;
use crate::shared::message::Query;

/// Surfaces eligible for resume: a kept record that is not live (idle/failed/pending).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResumableSurfaces;

impl Query<Ctx> for ListResumableSurfaces {
    type Out = Vec<SurfaceView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        Ok(sqlx::query_as::<_, SurfaceView>(
            "SELECT id, session_id, kind, cwd, status, placement, spawned_at
             FROM surface WHERE status != 'live'
             ORDER BY created_at ASC",
        )
        .fetch_all(cx.db())
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::test_util::{harness, one_surface, seed_session, spawn};
    use crate::entities::session::SessionId;
    use crate::entities::{SurfaceId, SurfaceKind, SurfaceStatus};
    use crate::infra::SurfaceRepo;

    // ListResumableSurfaces excludes live surfaces, includes stopped/idle ones
    #[tokio::test]
    async fn list_resumable_excludes_live_and_includes_idle() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-resume").await;
        h.bus.execute(spawn(&session)).await.unwrap();
        let live = one_surface(&h, &session).await;

        // a second surface, stopped -> idle, is resumable
        SurfaceRepo::create(
            &h.pool,
            Some("idle-one"),
            &SessionId::from_string(&session),
            SurfaceKind::Terminal,
            None,
            Some("slot-2"),
            SurfaceStatus::Pending,
        )
        .await
        .unwrap();
        SurfaceRepo::update_status(
            &h.pool,
            &SurfaceId::from_string("idle-one"),
            SurfaceStatus::Idle,
        )
        .await
        .unwrap();

        let resumable = h.bus.query(ListResumableSurfaces).await.unwrap();
        let ids: Vec<&str> = resumable.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"idle-one"));
        assert!(
            !ids.contains(&live.id.as_str()),
            "live surface is not resumable"
        );
    }
}
