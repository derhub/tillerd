use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;

/// Set a session's sort_order.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderSession {
    pub id: String,
    pub sort_order: u32,
}

impl Command<Ctx> for ReorderSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = SessionId::from_string(&self.id);
        let mut s = SessionRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.clone()))?;
        s.sort_order = self.sort_order;
        SessionRepo::update(cx.db(), &s).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::test_util::{create_one, ctx};

    #[tokio::test]
    async fn reorder_session_persists_sort_order() {
        let (bus, pool) = ctx().await;
        let id = create_one(&bus).await;

        bus.execute(ReorderSession {
            id: id.clone(),
            sort_order: 42,
        })
        .await
        .unwrap();

        let sort_order: i64 = sqlx::query_scalar("SELECT sort_order FROM session WHERE id = ?")
            .bind(&id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(sort_order, 42);
    }
}
