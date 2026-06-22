use crate::context::Ctx;
use crate::entities::session::Session;
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Query;
use crate::shared::errors::Result;
use crate::shared::pagination::{Listing, Page};

/// List every session across all projects, pinned-first. The sidebar groups
/// these by project, so it loads them in one call (no project filter).
pub struct ListAllSessions {
    pub page: Page,
}

impl Query<Ctx> for ListAllSessions {
    type Out = Listing<Session>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        SessionRepo::list_all(cx.db(), self.page.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::test_util::*;
    use crate::app::session::NewSessionCmd;

    #[tokio::test]
    async fn list_all_returns_every_session_unfiltered() {
        let (bus, _pool) = ctx().await;
        bus.execute(NewSessionCmd(draft(unfiled()))).await.unwrap();
        let _ = create_one(&bus).await;

        let all = bus
            .query(ListAllSessions { page: Page::All })
            .await
            .unwrap();
        assert!(
            all.items.len() >= 2,
            "list_all must return all sessions, got {}",
            all.items.len()
        );
    }
}
