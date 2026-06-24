use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::infra::NotificationRepo;
use crate::shared::message::Query;
use crate::shared::Result;

/// Count of unread notifications (badge count).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CountUnreadNotifications;

impl Query<Ctx> for CountUnreadNotifications {
    type Out = i64;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        NotificationRepo::count_unread(cx.db()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::notification::mark_notification_read::MarkNotificationRead;
    use crate::app::notification::test_util::*;
    use crate::shared::Bus;

    #[tokio::test]
    async fn count_unread_reflects_only_unread_records() {
        let bus = Bus::new(test_ctx().await);
        bus.execute(record_cmd("u1")).await.unwrap();
        bus.execute(record_cmd("u2")).await.unwrap();
        bus.execute(record_cmd("u3")).await.unwrap();

        bus.execute(MarkNotificationRead { id: "u1".into() })
            .await
            .unwrap();

        let count = bus.query(CountUnreadNotifications).await.unwrap();
        assert_eq!(count, 2);
    }
}
