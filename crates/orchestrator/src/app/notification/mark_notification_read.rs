use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::infra::NotificationRepo;
use crate::shared::message::Command;
use crate::shared::Result;

/// Mark one notification as read.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarkNotificationRead {
    pub id: String,
}

impl Command<Ctx> for MarkNotificationRead {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        NotificationRepo::mark_read(cx.db(), &self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::notification::count_unread_notifications::CountUnreadNotifications;
    use crate::app::notification::test_util::*;
    use crate::shared::Bus;

    // -- Scenario: marking read clears the unread badge ------------------------

    #[tokio::test]
    async fn mark_notification_read_removes_it_from_unread_listing() {
        let bus = Bus::new(test_ctx().await);
        bus.execute(record_cmd("r1")).await.unwrap();
        bus.execute(record_cmd("r2")).await.unwrap();

        bus.execute(MarkNotificationRead { id: "r1".into() })
            .await
            .unwrap();

        let unread = bus.query(list_unread_all()).await.unwrap();
        assert_eq!(unread.items.len(), 1);
        assert_eq!(unread.items[0].id, "r2");

        let count = bus.query(CountUnreadNotifications).await.unwrap();
        assert_eq!(count, 1);
    }

    // -- Scenario: mark_read on absent id returns not_found --------------------

    #[tokio::test]
    async fn mark_notification_read_on_absent_id_returns_not_found() {
        let bus = Bus::new(test_ctx().await);
        let err = bus
            .execute(MarkNotificationRead { id: "ghost".into() })
            .await
            .unwrap_err();
        assert_eq!(err.code(), "notification.not_found");
    }
}
