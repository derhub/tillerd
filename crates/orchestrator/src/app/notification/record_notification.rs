use crate::context::Ctx;
use crate::entities::notification::NotificationRecord;
use crate::infra::NotificationRepo;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Post a new notification record.
// NotificationRecord does not yet derive Serialize/Deserialize; the transport
// layer builds the command value after deserialization from the wire.
#[derive(Clone, Debug)]
pub struct RecordNotification {
    pub notification: NotificationRecord,
}

impl Command<Ctx> for RecordNotification {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        NotificationRepo::create(cx.db(), &self.notification).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::notification::list_notifications::ListNotifications;
    use crate::app::notification::test_util::*;
    use crate::shared::pagination::Page;
    use crate::shared::Bus;

    // ── Scenario: command mutates and returns nothing ─────────────────────────

    #[tokio::test]
    async fn record_notification_persists_and_returns_unit() {
        let bus = Bus::new(test_ctx().await);
        let n = sample("n1");

        // command returns nothing
        bus.execute(RecordNotification {
            notification: n.clone(),
        })
        .await
        .unwrap();

        // observable via query: the record is in the listing
        let listing = bus
            .query(ListNotifications { page: Page::All })
            .await
            .unwrap();
        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].id, "n1");
    }
}
