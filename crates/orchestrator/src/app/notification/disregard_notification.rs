use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::infra::NotificationRepo;
use crate::shared::message::Command;
use crate::shared::Result;

/// Delete a single notification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisregardNotification {
    pub id: String,
}

impl Command<Ctx> for DisregardNotification {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        NotificationRepo::delete(cx.db(), &self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::notification::test_util::*;
    use crate::shared::Bus;

    // -- Scenario: disregard removes a notification ----------------------------

    #[tokio::test]
    async fn disregard_notification_deletes_the_record() {
        let bus = Bus::new(test_ctx().await);
        bus.execute(record_cmd("d1")).await.unwrap();

        bus.execute(DisregardNotification { id: "d1".into() })
            .await
            .unwrap();

        let listing = bus.query(list_all()).await.unwrap();
        assert!(listing.items.is_empty());
    }

    // -- Scenario: disregard on absent id returns not_found --------------------

    #[tokio::test]
    async fn disregard_notification_on_absent_id_returns_not_found() {
        let bus = Bus::new(test_ctx().await);
        let err = bus
            .execute(DisregardNotification { id: "ghost".into() })
            .await
            .unwrap_err();
        assert_eq!(err.code(), "notification.not_found");
    }
}
