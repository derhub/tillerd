use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::entities::workspace::{Workspace, WorkspaceId};
use crate::infra::WorkspaceRepo;
use crate::shared::cqs::Query;
use crate::shared::Result;

/// Fetch one workspace by id. Returns `None` when absent.
#[derive(Debug, Serialize, Deserialize)]
pub struct GetWorkspaceById {
    pub id: WorkspaceId,
}

impl Query<Ctx> for GetWorkspaceById {
    type Out = Option<Workspace>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        WorkspaceRepo::get(cx.db(), &self.id).await
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
            id: ws_id("no-such"),
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
            id: ws_id("ws-get-1"),
        }
        .handle(&cx)
        .await
        .unwrap();
        assert!(out.is_some());
        assert_eq!(out.unwrap().name, "Beta");
    }
}
