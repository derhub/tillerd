use crate::context::Ctx;
use crate::entities::session::{SessionId, SessionStatus};
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::{Error, Result};

/// Restore an archived session to active.
pub struct RestoreSession {
    pub id: SessionId,
}

impl Command<Ctx> for RestoreSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let s = SessionRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.as_str().to_owned()))?;

        if s.status != SessionStatus::Archived {
            return Err(Error::SessionNotArchived);
        }

        SessionRepo::set_active(cx.db(), &self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::archive_session::ArchiveSession;
    use crate::app::session::get_session_by_id::GetSessionById;
    use crate::app::session::test_util::{create_one, ctx};

    // Scenario: An archived entity is restored
    #[tokio::test]
    async fn restore_session_returns_it_to_active() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;
        bus.execute(ArchiveSession { id: id.clone() })
            .await
            .unwrap();

        bus.execute(RestoreSession { id: id.clone() })
            .await
            .unwrap();

        let s = bus.query(GetSessionById { id }).await.unwrap().unwrap();
        assert_eq!(s.status, SessionStatus::Active);
    }

    // Scenario: Restore targets only archived entities
    #[tokio::test]
    async fn restore_session_rejects_active_session() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;

        let err = bus.execute(RestoreSession { id }).await.unwrap_err();
        assert_eq!(err.code(), "session.not_archived");
    }
}
