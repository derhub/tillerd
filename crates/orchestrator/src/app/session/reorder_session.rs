use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::{Error, Result};

/// Set a session's sort_order.
pub struct ReorderSession {
    pub id: SessionId,
    pub sort_order: u32,
}

impl Command<Ctx> for ReorderSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut s = SessionRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.as_str().to_owned()))?;
        s.sort_order = self.sort_order;
        SessionRepo::update(cx.db(), &s).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::get_session_by_id::GetSessionById;
    use crate::app::session::test_util::{create_one, ctx};

    #[tokio::test]
    async fn reorder_session_persists_sort_order() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;

        bus.execute(ReorderSession {
            id: id.clone(),
            sort_order: 42,
        })
        .await
        .unwrap();

        let s = bus.query(GetSessionById { id }).await.unwrap().unwrap();
        assert_eq!(s.sort_order, 42);
    }
}
