use crate::context::Ctx;
use crate::shared::cqs::Command as BusCommand;
use crate::shared::Result;

use super::seed_prebuilt;

/// Upsert the built-in prebuilt commands. Idempotent: safe to call at boot
/// even after the migration has already seeded the rows.
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
    use crate::entities::command::CommandOrigin;
    use crate::shared::pagination::Page;
    use crate::shared::Bus;

    // ── Scenario: SeedCommands is idempotent ──────────────────────────────────

    #[tokio::test]
    async fn seed_commands_is_idempotent() {
        let bus = Bus::new(ctx().await);
        bus.execute(SeedCommands).await.unwrap();
        bus.execute(SeedCommands).await.unwrap();

        let prebuilt = bus
            .query(ListCommands {
                origin: Some(CommandOrigin::Prebuilt),
                page: Page::All,
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
