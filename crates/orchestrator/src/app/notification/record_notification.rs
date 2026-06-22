use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::notification::NotificationRecord;
use crate::infra::NotificationRepo;
use crate::shared::message::Command;
use crate::shared::Result;

/// Post a new notification record. Holds flat primitive fields; the
/// `NotificationRecord` entity is assembled inside `handle`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordNotification {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub title: Option<String>,
    pub message: String,
    pub detail: Option<String>,
    pub ts: i64,
    pub session_id: Option<String>,
    pub surface_id: Option<String>,
    pub actions_json: Option<String>,
    pub read: bool,
    pub snooze_until: Option<i64>,
}

impl Command<Ctx> for RecordNotification {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let record = NotificationRecord {
            id: self.id.clone(),
            category: self.category.clone(),
            severity: self.severity.clone(),
            title: self.title.clone(),
            message: self.message.clone(),
            detail: self.detail.clone(),
            ts: self.ts,
            session_id: self.session_id.clone(),
            surface_id: self.surface_id.clone(),
            actions_json: self.actions_json.clone(),
            read: self.read,
            snooze_until: self.snooze_until,
        };
        NotificationRepo::create(cx.db(), &record).await
    }
}

#[cfg(test)]
mod tests {
    use crate::app::notification::test_util::*;
    use crate::shared::Bus;

    // -- Scenario: command mutates and returns nothing -------------------------

    #[tokio::test]
    async fn record_notification_persists_and_returns_unit() {
        let bus = Bus::new(test_ctx().await);

        // command returns nothing
        bus.execute(record_cmd("n1")).await.unwrap();

        // observable via query: the record is in the listing
        let listing = bus.query(list_all()).await.unwrap();
        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].id, "n1");
    }
}
