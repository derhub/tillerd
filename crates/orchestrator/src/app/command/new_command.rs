use std::collections::HashMap;

use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::command::{Command, CommandId, CommandOrigin};
use crate::infra::CommandRepo;
use crate::shared::message::Command as BusCommand;
use crate::shared::Result;

/// Create a new custom library command.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCommand {
    pub name: String,
    pub cli: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl BusCommand<Ctx> for NewCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let command = Command {
            id: CommandId::mint(),
            name: self.name.clone(),
            origin: CommandOrigin::Custom,
            cli: self.cli.clone(),
            args: self.args.clone(),
            env: self.env.clone(),
            pinned: false,
        };
        CommandRepo::create(cx.db(), &command).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::app::command::list_commands::ListCommands;
    use crate::app::command::test_util::*;
    use crate::shared::Bus;

    // -- Scenario: command mutates and returns nothing -------------------------

    #[tokio::test]
    async fn new_command_creates_a_custom_command_that_get_query_resolves() {
        let bus = Bus::new(ctx().await);
        bus.execute(NewCommand {
            name: "my-tool".to_owned(),
            cli: "/usr/bin/my-tool".to_owned(),
            args: vec!["--flag".to_owned()],
            env: HashMap::new(),
        })
        .await
        .unwrap();

        let listing = bus
            .query(ListCommands {
                origin: Some("custom".to_owned()),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        let found = listing.items.iter().find(|c| c.name == "my-tool").unwrap();
        assert_eq!(found.cli, "/usr/bin/my-tool");
        assert_eq!(found.args, vec!["--flag"]);
        assert_eq!(found.origin, "custom");
    }
}
