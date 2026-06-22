use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::{SessionId, SessionStatus};
use crate::infra::session::SessionRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;

/// Restore an archived session to active.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSession {
    pub id: String,
}

impl Command<Ctx> for RestoreSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = SessionId::from_string(&self.id);
        let s = SessionRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.clone()))?;

        if s.status != SessionStatus::Archived {
            return Err(Error::SessionNotArchived);
        }

        SessionRepo::set_active(cx.db(), &id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::archive_session::ArchiveSession;
    use crate::app::session::test_util::{create_one, ctx};

    async fn is_archived(pool: &sqlx::SqlitePool, id: &str) -> bool {
        let v: Option<String> = sqlx::query_scalar("SELECT archived_at FROM session WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();
        v.is_some()
    }

    // Scenario: An archived entity is restored
    #[tokio::test]
    async fn restore_session_returns_it_to_active() {
        let (bus, pool) = ctx().await;
        let id = create_one(&bus).await;
        bus.execute(ArchiveSession { id: id.clone() })
            .await
            .unwrap();

        bus.execute(RestoreSession { id: id.clone() })
            .await
            .unwrap();

        assert!(!is_archived(&pool, &id).await);
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
