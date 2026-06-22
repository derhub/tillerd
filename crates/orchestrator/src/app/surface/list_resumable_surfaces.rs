use crate::context::Ctx;
use crate::entities::Surface;
use crate::shared::cqs::Query;
use crate::shared::errors::Result;

use super::common::row_to_surface;

/// Surfaces eligible for resume: a kept record that is not live (idle/failed/pending).
#[derive(Debug, Clone, Default)]
pub struct ListResumableSurfaces;

impl Query<Ctx> for ListResumableSurfaces {
    type Out = Vec<Surface>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let rows = sqlx::query(
            "SELECT id, session_id, kind, cwd, placement, status
             FROM surface WHERE status != 'live'
             ORDER BY created_at ASC",
        )
        .fetch_all(cx.db())
        .await?;
        rows.into_iter().map(row_to_surface).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::test_util::{harness, one_surface, seed_session, spawn};
    use crate::entities::{NewSurface, SurfaceId, SurfaceKind, SurfaceStatus};
    use crate::infra::SurfaceRepo;

    // ListResumableSurfaces excludes live surfaces, includes stopped/idle ones
    #[tokio::test]
    async fn list_resumable_excludes_live_and_includes_idle() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-resume").await;
        h.bus.execute(spawn(&session)).await.unwrap();
        let live = one_surface(&h, &session).await;

        // a second surface, stopped → idle, is resumable
        SurfaceRepo::create(
            &h.pool,
            &NewSurface {
                id: Some(SurfaceId::from_string("idle-one")),
                session_id: session.clone(),
                kind: SurfaceKind::Terminal,
                cwd: None,
                placement: Some("slot-2".to_owned()),
            },
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
