use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;

/// Set the `pinned` flag to true.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinSession {
    pub id: String,
}

impl Command<Ctx> for PinSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = SessionId::from_string(&self.id);
        let mut s = SessionRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.clone()))?;
        s.pinned = true;
        SessionRepo::update(cx.db(), &s).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::test_util::{create_one, ctx};

    async fn pinned_flag(pool: &sqlx::SqlitePool, id: &str) -> bool {
        let v: i64 = sqlx::query_scalar("SELECT pinned FROM session WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();
        v != 0
    }

    // Scenario: pinning
    #[tokio::test]
    async fn pin_session_sets_pinned_flag() {
        let (bus, pool) = ctx().await;
        let id = create_one(&bus).await;
        bus.execute(PinSession { id: id.clone() }).await.unwrap();
        assert!(pinned_flag(&pool, &id).await);
    }
}
