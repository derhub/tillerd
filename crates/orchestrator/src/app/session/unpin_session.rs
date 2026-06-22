use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::{Error, Result};

/// Clear the `pinned` flag.
pub struct UnpinSession {
    pub id: SessionId,
}

impl Command<Ctx> for UnpinSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut s = SessionRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.as_str().to_owned()))?;
        s.pinned = false;
        SessionRepo::update(cx.db(), &s).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::get_session_by_id::GetSessionById;
    use crate::app::session::pin_session::PinSession;
    use crate::app::session::test_util::{create_one, ctx};

    #[tokio::test]
    async fn unpin_session_clears_pinned_flag() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;
        bus.execute(PinSession { id: id.clone() }).await.unwrap();
        bus.execute(UnpinSession { id: id.clone() }).await.unwrap();
        let s = bus.query(GetSessionById { id }).await.unwrap().unwrap();
        assert!(!s.pinned);
    }
}
