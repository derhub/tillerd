use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;

/// Clear the `pinned` flag.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnpinSession {
    pub id: String,
}

impl Command<Ctx> for UnpinSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = SessionId::from_string(&self.id);
        let mut s = SessionRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.clone()))?;
        s.pinned = false;
        SessionRepo::update(cx.db(), &s).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::pin_session::PinSession;
    use crate::app::session::test_util::{create_one, ctx};

    async fn pinned_flag(pool: &sqlx::SqlitePool, id: &str) -> bool {
        let v: i64 = sqlx::query_scalar("SELECT pinned FROM session WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();
        v != 0
    }

    #[tokio::test]
    async fn unpin_session_clears_pinned_flag() {
        let (bus, pool) = ctx().await;
        let id = create_one(&bus).await;
        bus.execute(PinSession { id: id.clone() }).await.unwrap();
        bus.execute(UnpinSession { id: id.clone() }).await.unwrap();
        assert!(!pinned_flag(&pool, &id).await);
    }
}
