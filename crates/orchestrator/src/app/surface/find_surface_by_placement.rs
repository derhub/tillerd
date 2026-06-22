use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::entities::Surface;
use crate::infra::SurfaceRepo;
use crate::shared::cqs::Query;
use crate::shared::errors::Result;

/// The surface bound to a session + placement slot.
#[derive(Debug, Clone)]
pub struct FindSurfaceByPlacement {
    pub session: SessionId,
    pub placement: String,
}

impl Query<Ctx> for FindSurfaceByPlacement {
    type Out = Option<Surface>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        SurfaceRepo::find_by_placement(cx.db(), &self.session, &self.placement).await
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
}
