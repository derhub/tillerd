use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::infra::NotificationRepo;
use crate::shared::message::Command;
use crate::shared::Result;

/// Set `snooze_until` on one notification (epoch milliseconds). Pass `None` to clear.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnoozeNotification {
    pub id: String,
    pub snooze_until: Option<i64>,
}

impl Command<Ctx> for SnoozeNotification {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        NotificationRepo::snooze(cx.db(), &self.id, self.snooze_until).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::notification::test_util::*;
    use crate::infra::NotificationRepo;
    use crate::shared::Bus;

    #[tokio::test]
    async fn snooze_sets_snooze_until_and_clear_restores_none() {
        let bus = Bus::new(test_ctx().await);
        bus.execute(record_cmd("sn1")).await.unwrap();

        bus.execute(SnoozeNotification {
            id: "sn1".into(),
            snooze_until: Some(99_999),
        })
        .await
        .unwrap();

        // observable via repo directly -- record must have the timestamp set
        let pool = bus.cx().db().clone();
        let rec = NotificationRepo::get(&pool, "sn1").await.unwrap().unwrap();
        assert_eq!(rec.snooze_until, Some(99_999));

        bus.execute(SnoozeNotification {
            id: "sn1".into(),
            snooze_until: None,
        })
        .await
        .unwrap();

        let cleared = NotificationRepo::get(&pool, "sn1").await.unwrap().unwrap();
        assert_eq!(cleared.snooze_until, None);
    }

    #[tokio::test]
    async fn snooze_on_absent_id_returns_not_found() {
        let bus = Bus::new(test_ctx().await);
        let err = bus
            .execute(SnoozeNotification {
                id: "ghost".into(),
                snooze_until: Some(1),
            })
            .await
            .unwrap_err();
        assert_eq!(err.code(), "notification.not_found");
    }
}
