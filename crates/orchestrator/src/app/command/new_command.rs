use std::collections::HashMap;

use crate::context::Ctx;
use crate::entities::command::{CommandOrigin, NewCommand as NewCommandDraft};
use crate::infra::CommandRepo;
use crate::shared::cqs::Command as BusCommand;
use crate::shared::Result;

/// Create a new custom library command.
pub struct NewCommand {
    pub name: String,
    pub cli: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

impl BusCommand<Ctx> for NewCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let draft = NewCommandDraft {
            name: self.name.clone(),
            origin: CommandOrigin::Custom,
            cli: self.cli.clone(),
            args: self.args.clone(),
            env: self.env.clone(),
        };
        CommandRepo::create(cx.db(), &draft).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::app::command::list_commands::ListCommands;
    use crate::app::command::test_util::*;
    use crate::entities::command::CommandOrigin;
    use crate::shared::pagination::Page;
    use crate::shared::Bus;

    // ── Scenario: command mutates and returns nothing ─────────────────────────

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
                origin: Some(CommandOrigin::Custom),
                page: Page::All,
            })
            .await
            .unwrap();
        let found = listing.items.iter().find(|c| c.name == "my-tool").unwrap();
        assert_eq!(found.cli, "/usr/bin/my-tool");
        assert_eq!(found.args, vec!["--flag"]);
        assert_eq!(found.origin, CommandOrigin::Custom);
    }
}
