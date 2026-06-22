use serde::Deserialize;

use crate::app::workspace::WorkspaceView;
use crate::context::Ctx;
use crate::shared::message::Query;
use crate::shared::Result;

/// Fetch one workspace by id. Returns `None` when absent.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWorkspaceById {
    pub id: String,
}

impl Query<Ctx> for GetWorkspaceById {
    type Out = Option<WorkspaceView>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        Ok(
            sqlx::query_as::<_, WorkspaceView>("SELECT id, name FROM workspace WHERE id = ?")
                .bind(&self.id)
                .fetch_optional(cx.db())
                .await?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::test_util::*;

    // Scenario: A query reads and does not mutate.
    #[tokio::test]
    async fn get_workspace_by_id_returns_none_for_absent() {
        let cx = ctx().await;
        let out = GetWorkspaceById {
            id: "no-such".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();
        assert!(out.is_none());
    }

    #[tokio::test]
    async fn get_workspace_by_id_returns_the_workspace() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-get-1", "Beta").await;
        let out = GetWorkspaceById {
            id: "ws-get-1".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();
        assert!(out.is_some());
        assert_eq!(out.unwrap().name, "Beta");
    }
}
