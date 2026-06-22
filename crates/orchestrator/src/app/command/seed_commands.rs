use serde::Deserialize;

use crate::context::Ctx;
use crate::shared::message::Command as BusCommand;
use crate::shared::Result;

use super::seed_prebuilt;

/// Upsert the built-in prebuilt commands. Idempotent: safe to call at boot
/// even after the migration has already seeded the rows.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedCommands;

impl BusCommand<Ctx> for SeedCommands {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        seed_prebuilt(cx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::command::list_commands::ListCommands;
    use crate::app::command::test_util::*;
    use crate::shared::Bus;

    // -- Scenario: SeedCommands is idempotent ----------------------------------

    #[tokio::test]
    async fn seed_commands_is_idempotent() {
        let bus = Bus::new(ctx().await);
        bus.execute(SeedCommands).await.unwrap();
        bus.execute(SeedCommands).await.unwrap();

        let prebuilt = bus
            .query(ListCommands {
                origin: Some("prebuilt".to_owned()),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        // Must not duplicate; login-shell appears exactly once.
        let count = prebuilt
            .items
            .iter()
            .filter(|c| c.name == "login-shell")
            .count();
        assert_eq!(count, 1);
    }
}
