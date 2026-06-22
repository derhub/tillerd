use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::{Error, Result};

/// Set the `pinned` flag to true.
pub struct PinSession {
    pub id: SessionId,
}

impl Command<Ctx> for PinSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut s = SessionRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.as_str().to_owned()))?;
        s.pinned = true;
        SessionRepo::update(cx.db(), &s).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::get_session_by_id::GetSessionById;
    use crate::app::session::test_util::{create_one, ctx};

    // Scenario: pinning
    #[tokio::test]
    async fn pin_session_sets_pinned_flag() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;
        bus.execute(PinSession { id: id.clone() }).await.unwrap();
        let s = bus.query(GetSessionById { id }).await.unwrap().unwrap();
        assert!(s.pinned);
    }
}
