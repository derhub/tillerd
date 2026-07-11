use serde::Deserialize;

use crate::app::surface::SurfaceView;
use crate::context::Ctx;
use crate::shared::errors::Result;
use crate::shared::message::Query;

/// The surface bound to a session + placement slot.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindSurfaceByPlacement {
    pub session: String,
    pub placement: String,
}

impl Query<Ctx> for FindSurfaceByPlacement {
    type Out = Option<SurfaceView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        Ok(sqlx::query_as::<_, SurfaceView>(
            "SELECT id, session_id, kind, cwd, status, placement, spawned_at
             FROM surface WHERE session_id = ? AND placement = ?",
        )
        .bind(&self.session)
        .bind(&self.placement)
        .fetch_optional(cx.db())
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::test_util::{harness, seed_session, spawn};

    // Scenario: A query reads and does not mutate
    #[tokio::test]
    async fn find_by_placement_resolves_a_minted_slot() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-find").await;
        h.bus.execute(spawn(&session)).await.unwrap();

        let found = h
            .bus
            .query(FindSurfaceByPlacement {
                session: session.clone(),
                placement: "main".to_owned(),
            })
            .await
            .unwrap();
        assert!(found.is_some());

        let none = h
            .bus
            .query(FindSurfaceByPlacement {
                session,
                placement: "unused".to_owned(),
            })
            .await
            .unwrap();
        assert!(none.is_none());
    }

    // Scenario: the view carries the spawn timestamp once the PTY is confirmed.
    #[tokio::test]
    async fn find_by_placement_exposes_spawned_at_after_spawn() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-find-spawned-at").await;
        h.bus.execute(spawn(&session)).await.unwrap();

        let found = h
            .bus
            .query(FindSurfaceByPlacement {
                session,
                placement: "main".to_owned(),
            })
            .await
            .unwrap()
            .expect("a surface");
        assert!(found.spawned_at.is_some());
    }
}
