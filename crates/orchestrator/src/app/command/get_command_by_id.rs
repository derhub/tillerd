use serde::Deserialize;

use crate::app::command::CommandView;
use crate::context::Ctx;
use crate::shared::message::Query;
use crate::shared::Result;

/// Fetch a single library command by id.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCommandById {
    pub id: String,
}

impl Query<Ctx> for GetCommandById {
    type Out = Option<CommandView>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        Ok(sqlx::query_as::<_, CommandView>(
            "SELECT id, name, origin, cli, args_json, env_json
             FROM command
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(&self.id)
        .fetch_optional(cx.db())
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::command::test_util::*;
    use crate::shared::Bus;

    #[tokio::test]
    async fn get_command_by_id_returns_none_for_absent_id() {
        let bus = Bus::new(ctx().await);
        let result = bus
            .query(GetCommandById {
                id: "no-such-id".to_owned(),
            })
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
