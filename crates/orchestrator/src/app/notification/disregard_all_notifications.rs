use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::infra::NotificationRepo;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Delete all notifications.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisregardAllNotifications;

impl Command<Ctx> for DisregardAllNotifications {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        NotificationRepo::delete_all(cx.db()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::notification::list_notifications::ListNotifications;
    use crate::app::notification::record_notification::RecordNotification;
    use crate::app::notification::test_util::*;
    use crate::shared::pagination::Page;
    use crate::shared::Bus;

    #[tokio::test]
    async fn disregard_all_notifications_clears_the_store() {
        let bus = Bus::new(test_ctx().await);
        for id in ["x", "y", "z"] {
            bus.execute(RecordNotification {
                notification: sample(id),
            })
            .await
            .unwrap();
        }

        bus.execute(DisregardAllNotifications).await.unwrap();

        let listing = bus
            .query(ListNotifications { page: Page::All })
            .await
            .unwrap();
        assert!(listing.items.is_empty());
    }
}
