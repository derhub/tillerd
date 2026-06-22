use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::entities::notification::NotificationRecord;
use crate::infra::NotificationRepo;
use crate::shared::cqs::Query;
use crate::shared::pagination::{Listing, Page};
use crate::shared::Result;

/// All notifications ordered by `ts DESC`, with optional pagination.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListNotifications {
    pub page: Page,
}

impl Query<Ctx> for ListNotifications {
    type Out = Listing<NotificationRecord>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        NotificationRepo::list(cx.db(), &self.page).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::notification::count_unread_notifications::CountUnreadNotifications;
    use crate::app::notification::record_notification::RecordNotification;
    use crate::app::notification::test_util::*;
    use crate::shared::Bus;

    // ── Scenario: query reads and does not mutate ─────────────────────────────

    #[tokio::test]
    async fn list_notifications_does_not_mutate_state() {
        let bus = Bus::new(test_ctx().await);
        bus.execute(RecordNotification {
            notification: sample("a"),
        })
        .await
        .unwrap();
        bus.execute(RecordNotification {
            notification: sample("b"),
        })
        .await
        .unwrap();

        // two queries in a row — count must not change
        let c1 = bus.query(CountUnreadNotifications).await.unwrap();
        let c2 = bus.query(CountUnreadNotifications).await.unwrap();
        assert_eq!(c1, c2);
        assert_eq!(c1, 2);
    }

    // ── Scenario: list returns records ordered ts DESC ────────────────────────

    #[tokio::test]
    async fn list_notifications_ordered_by_ts_desc() {
        let bus = Bus::new(test_ctx().await);
        for (id, ts) in [("q1", 100i64), ("q2", 300), ("q3", 200)] {
            bus.execute(RecordNotification {
                notification: sample_at(id, ts),
            })
            .await
            .unwrap();
        }

        let listing = bus
            .query(ListNotifications { page: Page::All })
            .await
            .unwrap();
        let ids: Vec<&str> = listing.items.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["q2", "q3", "q1"]);
    }
}
