use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::infra::NotificationRepo;
use crate::shared::message::Command;
use crate::shared::Result;

/// Retention cap: keep only the most recent `keep` records.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PruneNotifications {
    pub keep: u32,
}

impl Command<Ctx> for PruneNotifications {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        NotificationRepo::prune(cx.db(), self.keep).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::notification::test_util::*;
    use crate::shared::Bus;

    // -- Scenario: prune keeps only the most recent N records ------------------

    #[tokio::test]
    async fn prune_notifications_keeps_only_the_most_recent_n() {
        let bus = Bus::new(test_ctx().await);
        for (id, ts) in [
            ("p1", 10i64),
            ("p2", 20),
            ("p3", 30),
            ("p4", 40),
            ("p5", 50),
        ] {
            bus.execute(record_cmd_at(id, ts)).await.unwrap();
        }

        bus.execute(PruneNotifications { keep: 3 }).await.unwrap();

        let listing = bus.query(list_all()).await.unwrap();
        assert_eq!(listing.items.len(), 3);
        let ids: Vec<&str> = listing.items.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["p5", "p4", "p3"]);
    }
}
