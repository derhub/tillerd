use crate::context::Ctx;
use crate::entities::command::CommandId;
use crate::infra::CommandRepo;
use crate::shared::cqs::Command as BusCommand;
use crate::shared::{Error, Result};

use super::guard_not_prebuilt;

/// Hard-delete (soft-delete via `deleted_at`) a custom command.
pub struct DiscardCommand {
    pub id: CommandId,
}

impl BusCommand<Ctx> for DiscardCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let cmd = CommandRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::CommandNotFound(self.id.as_str().to_owned()))?;
        guard_not_prebuilt(&cmd)?;
        CommandRepo::delete(cx.db(), &self.id).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::app::command::get_command_by_id::GetCommandById;
    use crate::app::command::list_commands::ListCommands;
    use crate::app::command::new_command::NewCommand;
    use crate::app::command::test_util::*;
    use crate::entities::command::CommandOrigin;
    use crate::shared::pagination::Page;
    use crate::shared::Bus;

    // ── Scenario: prebuilt commands are immutable ─────────────────────────────

    #[tokio::test]
    async fn discard_command_rejects_prebuilt() {
        let bus = Bus::new(ctx().await);
        let prebuilt_id = bus
            .query(ListCommands {
                origin: Some(CommandOrigin::Prebuilt),
                page: Page::All,
            })
            .await
            .unwrap()
            .items
            .into_iter()
            .next()
            .unwrap()
            .id;

        let err = bus
            .execute(DiscardCommand { id: prebuilt_id })
            .await
            .unwrap_err();
        assert_eq!(err.code(), "prebuilt.immutable");
    }

    // ── Scenario: discard removes a custom command ────────────────────────────

    #[tokio::test]
    async fn discard_command_removes_it_from_list() {
        let bus = Bus::new(ctx().await);
        bus.execute(NewCommand {
            name: "to-discard".to_owned(),
            cli: "/bin/gone".to_owned(),
            args: vec![],
            env: HashMap::new(),
        })
        .await
        .unwrap();
        let id = bus
            .query(ListCommands {
                origin: Some(CommandOrigin::Custom),
                page: Page::All,
            })
            .await
            .unwrap()
            .items
            .into_iter()
            .find(|c| c.name == "to-discard")
            .unwrap()
            .id;

        bus.execute(DiscardCommand { id: id.clone() })
            .await
            .unwrap();

        let result = bus.query(GetCommandById { id }).await.unwrap();
        assert!(result.is_none());
    }

    // ── Scenario: not-found errors ────────────────────────────────────────────

    #[tokio::test]
    async fn discard_command_returns_not_found_for_absent_id() {
        let bus = Bus::new(ctx().await);
        let err = bus
            .execute(DiscardCommand {
                id: CommandId::from_string("ghost"),
            })
            .await
            .unwrap_err();
        assert_eq!(err.code(), "command.not_found");
    }
}
