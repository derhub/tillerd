use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::infra::NotificationRepo;
use crate::shared::message::Command;
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
    use crate::app::notification::test_util::*;
    use crate::shared::Bus;

    #[tokio::test]
    async fn disregard_all_notifications_clears_the_store() {
        let bus = Bus::new(test_ctx().await);
        for id in ["x", "y", "z"] {
            bus.execute(record_cmd(id)).await.unwrap();
        }

        bus.execute(DisregardAllNotifications).await.unwrap();

        let listing = bus.query(list_all()).await.unwrap();
        assert!(listing.items.is_empty());
    }
}
