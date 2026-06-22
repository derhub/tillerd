use crate::context::Ctx;
use crate::entities::command::CommandId;
use crate::infra::CommandRepo;
use crate::shared::cqs::Command as BusCommand;
use crate::shared::Result;

/// Pin a library command (favorite); pinned commands sort before unpinned ones.
pub struct PinCommand {
    pub id: CommandId,
}

impl BusCommand<Ctx> for PinCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        CommandRepo::set_pinned(cx.db(), &self.id, true).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::app::command::list_commands::ListCommands;
    use crate::app::command::new_command::NewCommand;
    use crate::app::command::test_util::*;
    use crate::entities::command::CommandOrigin;
    use crate::shared::pagination::Page;
    use crate::shared::Bus;

    #[tokio::test]
    async fn pin_command_sets_pinned_true_and_list_returns_it_first() {
        let bus = Bus::new(ctx().await);
        bus.execute(NewCommand {
            name: "unpinned".to_owned(),
            cli: "/a".to_owned(),
            args: vec![],
            env: HashMap::new(),
        })
        .await
        .unwrap();
        bus.execute(NewCommand {
            name: "to-pin".to_owned(),
            cli: "/b".to_owned(),
            args: vec![],
            env: HashMap::new(),
        })
        .await
        .unwrap();

        let to_pin_id = bus
            .query(ListCommands {
                origin: Some(CommandOrigin::Custom),
                page: Page::All,
            })
            .await
            .unwrap()
            .items
            .into_iter()
            .find(|c| c.name == "to-pin")
            .unwrap()
            .id;

        bus.execute(PinCommand {
            id: to_pin_id.clone(),
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
        let first = &listing.items[0];
        assert_eq!(first.id, to_pin_id, "pinned command must sort first");
        assert!(first.pinned);
    }
}
