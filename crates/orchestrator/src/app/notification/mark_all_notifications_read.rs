use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::infra::NotificationRepo;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Mark every notification as read.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarkAllNotificationsRead;

impl Command<Ctx> for MarkAllNotificationsRead {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        NotificationRepo::mark_all_read(cx.db()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::notification::count_unread_notifications::CountUnreadNotifications;
    use crate::app::notification::list_notifications::ListNotifications;
    use crate::app::notification::list_unread_notifications::ListUnreadNotifications;
    use crate::app::notification::record_notification::RecordNotification;
    use crate::app::notification::test_util::*;
    use crate::shared::pagination::Page;
    use crate::shared::Bus;

    #[tokio::test]
    async fn mark_all_notifications_read_clears_badge_to_zero() {
        let bus = Bus::new(test_ctx().await);
        for id in ["a", "b", "c"] {
            bus.execute(RecordNotification {
                notification: sample(id),
            })
            .await
            .unwrap();
        }

        bus.execute(MarkAllNotificationsRead).await.unwrap();

        let count = bus.query(CountUnreadNotifications).await.unwrap();
        assert_eq!(count, 0);

        let unread = bus
            .query(ListUnreadNotifications { page: Page::All })
            .await
            .unwrap();
        assert!(unread.items.is_empty());
    }

    // ── Scenario: list unread excludes read records ───────────────────────────

    #[tokio::test]
    async fn list_unread_excludes_read_records() {
        let bus = Bus::new(test_ctx().await);
        bus.execute(RecordNotification {
            notification: sample("e1"),
        })
        .await
        .unwrap();
        bus.execute(RecordNotification {
            notification: sample("e2"),
        })
        .await
        .unwrap();

        bus.execute(MarkAllNotificationsRead).await.unwrap();

        let unread = bus
            .query(ListUnreadNotifications { page: Page::All })
            .await
            .unwrap();
        assert!(unread.items.is_empty());

        let all = bus
            .query(ListNotifications { page: Page::All })
            .await
            .unwrap();
        assert_eq!(all.items.len(), 2);
    }
}
