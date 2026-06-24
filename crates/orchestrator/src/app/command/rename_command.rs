use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::command::CommandId;
use crate::infra::CommandRepo;
use crate::shared::message::Command as BusCommand;
use crate::shared::{Error, Result};

use super::guard_not_prebuilt;

/// Rename a custom library command.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameCommand {
    pub id: String,
    pub name: String,
}

impl BusCommand<Ctx> for RenameCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = CommandId::from_string(&self.id);
        let mut cmd = CommandRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::CommandNotFound(self.id.clone()))?;
        guard_not_prebuilt(&cmd)?;
        cmd.rename(&self.name);
        CommandRepo::update(cx.db(), &cmd).await
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
    async fn rename_command_updates_the_name_and_returns_nothing() {
        let bus = Bus::new(ctx().await);
        bus.execute(NewCommand {
            id: uuid::Uuid::new_v4().to_string(),
            name: "orig".to_owned(),
            cli: "/bin/bash".to_owned(),
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
            .find(|c| c.name == "orig")
            .unwrap()
            .id;

        bus.execute(RenameCommand {
            id: id.clone(),
            name: "renamed".to_owned(),
        })
        .await
        .unwrap();

        let cmd = bus.query(GetCommandById { id }).await.unwrap().unwrap();
        assert_eq!(cmd.name, "renamed");
    }

    #[tokio::test]
    async fn rename_command_rejects_prebuilt() {
        let bus = Bus::new(ctx().await);
        // login-shell is seeded as prebuilt in the migration.
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
            .execute(RenameCommand {
                id: prebuilt_id,
                name: "hacked".to_owned(),
            })
            .await
            .unwrap_err();
        assert_eq!(err.code(), "prebuilt.immutable");
    }

    #[tokio::test]
    async fn rename_command_returns_not_found_for_absent_id() {
        let bus = Bus::new(ctx().await);
        let err = bus
            .execute(RenameCommand {
                id: "ghost".to_owned(),
                name: "x".to_owned(),
            })
            .await
            .unwrap_err();
        assert_eq!(err.code(), "command.not_found");
    }
}
