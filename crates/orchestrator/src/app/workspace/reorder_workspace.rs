use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::entities::workspace::WorkspaceId;
use crate::infra::WorkspaceRepo;
use crate::shared::cqs::Command;
use crate::shared::{Error, Result};

/// Set a workspace's sort order. Single-write.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReorderWorkspace {
    pub id: WorkspaceId,
    pub sort_order: u32,
}

impl Command<Ctx> for ReorderWorkspace {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut ws = WorkspaceRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::WorkspaceNotFound(self.id.as_str().to_owned()))?;
        ws.sort_order = self.sort_order;
        WorkspaceRepo::update(cx.db(), &ws).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::get_workspace_by_id::GetWorkspaceById;
    use crate::app::workspace::test_util::*;
    use crate::shared::cqs::Query;

    // Scenario: Reorder sets sort_order.
    #[tokio::test]
    async fn reorder_workspace_sets_sort_order() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-ord-1", "Orderable").await;
        ReorderWorkspace {
            id: ws_id("ws-ord-1"),
            sort_order: 42,
        }
        .handle(&cx)
        .await
        .unwrap();
        let ws = GetWorkspaceById {
            id: ws_id("ws-ord-1"),
        }
        .handle(&cx)
        .await
        .unwrap()
        .unwrap();
        assert_eq!(ws.sort_order, 42);
    }
}
