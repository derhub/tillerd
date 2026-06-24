use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::command::CommandId;
use crate::infra::CommandRepo;
use crate::shared::message::Command as BusCommand;
use crate::shared::{Error, Result};

use super::guard_not_prebuilt;

/// Hard-delete (soft-delete via `deleted_at`) a custom command.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardCommand {
    pub id: String,
}

impl BusCommand<Ctx> for DiscardCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = CommandId::from_string(&self.id);
        let cmd = CommandRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::CommandNotFound(self.id.clone()))?;
        guard_not_prebuilt(&cmd)?;
        CommandRepo::delete(cx.db(), &id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::command::get_command_by_id::GetCommandById;
    use crate::app::command::list_commands::ListCommands;
    use crate::app::command::new_command::NewCommand;
    use crate::app::command::test_util::*;
    use crate::shared::Bus;

    #[tokio::test]
    async fn discard_command_rejects_prebuilt() {
        let bus = Bus::new(ctx().await);
        let prebuilt_id = bus
            .query(ListCommands {
                origin: Some("prebuilt".to_owned()),
                limit: None,
                offset: None,
                after: None,
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

    #[tokio::test]
    async fn discard_command_removes_it_from_list() {
        let bus = Bus::new(ctx().await);
        bus.execute(NewCommand {
            id: uuid::Uuid::new_v4().to_string(),
            name: "to-discard".to_owned(),
            cli: "/bin/gone".to_owned(),
            args: vec![],
            env: std::collections::HashMap::new(),
        })
        .await
        .unwrap();
        let id = bus
            .query(ListCommands {
                origin: Some("custom".to_owned()),
                limit: None,
                offset: None,
                after: None,
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

    #[tokio::test]
    async fn discard_command_returns_not_found_for_absent_id() {
        let bus = Bus::new(ctx().await);
        let err = bus
            .execute(DiscardCommand {
                id: "ghost".to_owned(),
            })
            .await
            .unwrap_err();
        assert_eq!(err.code(), "command.not_found");
    }
}
