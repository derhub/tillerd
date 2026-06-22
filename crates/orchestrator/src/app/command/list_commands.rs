use crate::context::Ctx;
use crate::entities::command::{Command, CommandOrigin};
use crate::infra::CommandRepo;
use crate::shared::cqs::Query;
use crate::shared::pagination::{Listing, Page};
use crate::shared::Result;

/// List all library commands, optionally filtered by origin.
pub struct ListCommands {
    pub origin: Option<CommandOrigin>,
    pub page: Page,
}

impl Query<Ctx> for ListCommands {
    type Out = Listing<Command>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let origin_str = self.origin.as_ref().map(|o| o.as_str());
        CommandRepo::list(cx.db(), origin_str, self.page.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::app::command::new_command::NewCommand;
    use crate::app::command::test_util::*;
    use crate::shared::Bus;

    #[tokio::test]
    async fn list_commands_returns_all_including_seeded_prebuilt() {
        let bus = Bus::new(ctx().await);
        let listing = bus
            .query(ListCommands {
                origin: None,
                page: Page::All,
            })
            .await
            .unwrap();
        // The migration seeds login-shell as prebuilt.
        assert!(listing.items.iter().any(|c| c.name == "login-shell"));
    }

    #[tokio::test]
    async fn list_commands_by_origin_returns_only_matching_origin() {
        let bus = Bus::new(ctx().await);
        bus.execute(NewCommand {
            name: "custom-one".to_owned(),
            cli: "/bin/bash".to_owned(),
            args: vec![],
            env: HashMap::new(),
        })
        .await
        .unwrap();

        let custom_only = bus
            .query(ListCommands {
                origin: Some(CommandOrigin::Custom),
                page: Page::All,
            })
            .await
            .unwrap();
        assert!(custom_only
            .items
            .iter()
            .all(|c| c.origin == CommandOrigin::Custom));

        let prebuilt_only = bus
            .query(ListCommands {
                origin: Some(CommandOrigin::Prebuilt),
                page: Page::All,
            })
            .await
            .unwrap();
        assert!(prebuilt_only
            .items
            .iter()
            .all(|c| c.origin == CommandOrigin::Prebuilt));
    }
}
