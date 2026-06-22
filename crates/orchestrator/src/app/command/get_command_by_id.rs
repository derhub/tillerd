use crate::context::Ctx;
use crate::entities::command::{Command, CommandId};
use crate::infra::CommandRepo;
use crate::shared::cqs::Query;
use crate::shared::Result;

/// Fetch a single library command by id.
pub struct GetCommandById {
    pub id: CommandId,
}

impl Query<Ctx> for GetCommandById {
    type Out = Option<Command>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        CommandRepo::get(cx.db(), &self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::command::test_util::*;
    use crate::shared::Bus;

    // ── Scenario: query reads and does not mutate ─────────────────────────────

    #[tokio::test]
    async fn get_command_by_id_returns_none_for_absent_id() {
        let bus = Bus::new(ctx().await);
        let result = bus
            .query(GetCommandById {
                id: CommandId::from_string("no-such-id"),
            })
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
