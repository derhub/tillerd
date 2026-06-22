use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::command::CommandId;
use crate::infra::CommandRepo;
use crate::shared::message::Command as BusCommand;
use crate::shared::Result;

/// Unpin a library command; it returns to unpinned sort order.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnpinCommand {
    pub id: String,
}

impl BusCommand<Ctx> for UnpinCommand {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = CommandId::from_string(&self.id);
        CommandRepo::set_pinned(cx.db(), &id, false).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;
    use crate::app::command::list_commands::ListCommands;
    use crate::app::command::new_command::NewCommand;
    use crate::app::command::pin_command::PinCommand;
    use crate::context::Ctx;
    use crate::infra::migrate;
    use crate::infra::runtime::FakeRuntime;
    use crate::shared::kv::SqliteKv;
    use crate::shared::Bus;

    #[tokio::test]
    async fn unpin_command_clears_the_pinned_flag() {
        let pool = migrate::open_memory().await.unwrap();
        let kv = SqliteKv::in_memory().await.unwrap();
        let cx = Ctx::new(
            pool.clone(),
            kv,
            PathBuf::from("/tmp"),
            Arc::new(FakeRuntime::new()),
        );
        let bus = Bus::new(cx);
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
                origin: Some("custom".to_owned()),
                limit: None,
                offset: None,
                after: None,
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

        // CommandView does not expose `pinned`; verify the column directly.
        let pinned: bool = sqlx::query_scalar("SELECT pinned FROM command WHERE id = ?")
            .bind(&id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(!pinned, "unpin must clear the pinned flag");
    }
}
