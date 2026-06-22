use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::command::CommandId;
use crate::infra::CommandRepo;
use crate::shared::message::Command as BusCommand;
use crate::shared::Result;

/// Pin a library command (favorite); pinned commands sort before unpinned ones.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinCommand {
    pub id: String,
}

impl BusCommand<Ctx> for PinCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = CommandId::from_string(&self.id);
        CommandRepo::set_pinned(cx.db(), &id, true).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::app::command::list_commands::ListCommands;
    use crate::app::command::new_command::NewCommand;
    use crate::app::command::test_util::*;
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
                origin: Some("custom".to_owned()),
                limit: None,
                offset: None,
                after: None,
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
                origin: Some("custom".to_owned()),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        let first = &listing.items[0];
        // CommandView does not expose `pinned`; the sort-order contract (pinned DESC)
        // already proves the flag is set when this ID comes first.
        assert_eq!(first.id, to_pin_id, "pinned command must sort first");
    }
}
