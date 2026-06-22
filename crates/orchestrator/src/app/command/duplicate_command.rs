use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::command::{Command, CommandId, CommandOrigin};
use crate::infra::CommandRepo;
use crate::shared::message::Command as BusCommand;
use crate::shared::{Error, Result};

/// Clone a command as an editable `Custom` copy. Works on `Prebuilt` and `Custom`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCommand {
    pub id: String,
    pub name: String,
}

impl BusCommand<Ctx> for DuplicateCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = CommandId::from_string(&self.id);
        let src = CommandRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::CommandNotFound(self.id.clone()))?;
        let copy = Command {
            id: CommandId::mint(),
            name: self.name.clone(),
            origin: CommandOrigin::Custom,
            cli: src.cli,
            args: src.args,
            env: src.env,
            pinned: false,
        };
        CommandRepo::create(cx.db(), &copy).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::app::command::get_command_by_id::GetCommandById;
    use crate::app::command::list_commands::ListCommands;
    use crate::app::command::new_command::NewCommand;
    use crate::app::command::rename_command::RenameCommand;
    use crate::app::command::test_util::*;
    use crate::shared::Bus;

    // -- Scenario: duplicate prebuilt yields editable custom ------------------

    #[tokio::test]
    async fn duplicate_command_makes_editable_custom_copy_of_prebuilt() {
        let bus = Bus::new(ctx().await);
        let prebuilt = bus
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
            .unwrap();

        bus.execute(DuplicateCommand {
            id: prebuilt.id.clone(),
            name: "my-login-shell".to_owned(),
        })
        .await
        .unwrap();

        let copy = bus
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
            .find(|c| c.name == "my-login-shell")
            .unwrap();
        assert_eq!(copy.origin, "custom");
        assert_eq!(copy.cli, prebuilt.cli);
        assert_eq!(copy.args, prebuilt.args);

        // The copy is independently editable.
        bus.execute(RenameCommand {
            id: copy.id,
            name: "renamed-copy".to_owned(),
        })
        .await
        .unwrap();
    }

    // -- Scenario: duplicate custom yields independent copy --------------------

    #[tokio::test]
    async fn duplicate_custom_command_is_independent_of_source() {
        let bus = Bus::new(ctx().await);
        bus.execute(NewCommand {
            name: "src".to_owned(),
            cli: "/bin/src".to_owned(),
            args: vec!["--src".to_owned()],
            env: HashMap::new(),
        })
        .await
        .unwrap();
        let src_id = bus
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
            .find(|c| c.name == "src")
            .unwrap()
            .id;

        bus.execute(DuplicateCommand {
            id: src_id.clone(),
            name: "copy".to_owned(),
        })
        .await
        .unwrap();

        // Rename the copy; source is unchanged.
        let copy_id = bus
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
            .find(|c| c.name == "copy")
            .unwrap()
            .id;
        bus.execute(RenameCommand {
            id: copy_id,
            name: "copy-renamed".to_owned(),
        })
        .await
        .unwrap();

        let src = bus
            .query(GetCommandById { id: src_id })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(src.name, "src");
    }
}
