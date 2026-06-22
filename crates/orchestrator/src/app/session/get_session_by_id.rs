use crate::context::Ctx;
use crate::entities::session::{Session, SessionId};
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Query;
use crate::shared::errors::Result;

/// Fetch one session by id.
pub struct GetSessionById {
    pub id: SessionId,
}

impl Query<Ctx> for GetSessionById {
    type Out = Option<Session>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        SessionRepo::get(cx.db(), &self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::test_util::ctx;
    use crate::entities::session::SessionId;

    // Scenario: A query reads and does not mutate
    #[tokio::test]
    async fn get_session_by_id_returns_none_for_missing() {
        let (bus, _) = ctx().await;
        let result = bus
            .query(GetSessionById {
                id: SessionId::from_string("no-such"),
            })
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
