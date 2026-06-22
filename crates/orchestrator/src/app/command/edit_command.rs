use std::collections::HashMap;

use crate::context::Ctx;
use crate::entities::command::CommandId;
use crate::infra::CommandRepo;
use crate::shared::cqs::Command as BusCommand;
use crate::shared::{Error, Result};

use super::guard_not_prebuilt;

/// Replace a custom command's `cli`, `args`, and `env`.
pub struct EditCommand {
    pub id: CommandId,
    pub cli: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

impl BusCommand<Ctx> for EditCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut cmd = CommandRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::CommandNotFound(self.id.as_str().to_owned()))?;
        guard_not_prebuilt(&cmd)?;
        cmd.cli = self.cli.clone();
        cmd.args = self.args.clone();
        cmd.env = self.env.clone();
        CommandRepo::update(cx.db(), &cmd).await
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

    // ── Scenario: edit mutates cli/args/env ──────────────────────────────────

    #[tokio::test]
    async fn edit_command_updates_cli_args_env_and_returns_nothing() {
        let bus = Bus::new(ctx().await);
        bus.execute(NewCommand {
            name: "editable".to_owned(),
            cli: "/old".to_owned(),
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
            .find(|c| c.name == "editable")
            .unwrap()
            .id;

        let mut new_env = HashMap::new();
        new_env.insert("FOO".to_owned(), "bar".to_owned());
        bus.execute(EditCommand {
            id: id.clone(),
            cli: "/new".to_owned(),
            args: vec!["--verbose".to_owned()],
            env: new_env.clone(),
        })
        .await
        .unwrap();

        let cmd = bus.query(GetCommandById { id }).await.unwrap().unwrap();
        assert_eq!(cmd.cli, "/new");
        assert_eq!(cmd.args, vec!["--verbose"]);
        assert_eq!(cmd.env, new_env);
    }

    #[tokio::test]
    async fn edit_command_rejects_prebuilt() {
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
            .execute(EditCommand {
                id: prebuilt_id,
                cli: "/evil".to_owned(),
                args: vec![],
                env: HashMap::new(),
            })
            .await
            .unwrap_err();
        assert_eq!(err.code(), "prebuilt.immutable");
    }
}
