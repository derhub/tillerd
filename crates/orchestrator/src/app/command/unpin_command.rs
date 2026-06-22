use crate::context::Ctx;
use crate::entities::command::CommandId;
use crate::infra::CommandRepo;
use crate::shared::cqs::Command as BusCommand;
use crate::shared::Result;

/// Unpin a library command; it returns to unpinned sort order.
pub struct UnpinCommand {
    pub id: CommandId,
}

impl BusCommand<Ctx> for UnpinCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        CommandRepo::set_pinned(cx.db(), &self.id, false).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::app::command::get_command_by_id::GetCommandById;
    use crate::app::command::list_commands::ListCommands;
    use crate::app::command::new_command::NewCommand;
    use crate::app::command::pin_command::PinCommand;
    use crate::app::command::test_util::*;
    use crate::entities::command::CommandOrigin;
    use crate::shared::pagination::Page;
    use crate::shared::Bus;

    #[tokio::test]
    async fn unpin_command_clears_the_pinned_flag() {
        let bus = Bus::new(ctx().await);
        bus.execute(NewCommand {
            name: "pinned".to_owned(),
            cli: "/x".to_owned(),
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
            .find(|c| c.name == "pinned")
            .unwrap()
            .id;

        bus.execute(PinCommand { id: id.clone() }).await.unwrap();
        bus.execute(UnpinCommand { id: id.clone() }).await.unwrap();

        let cmd = bus.query(GetCommandById { id }).await.unwrap().unwrap();
        assert!(!cmd.pinned);
    }
}
