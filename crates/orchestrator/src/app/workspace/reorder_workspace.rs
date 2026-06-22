use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::workspace::WorkspaceId;
use crate::infra::WorkspaceRepo;
use crate::shared::message::Command;
use crate::shared::{Error, Result};

/// Set a workspace's sort order. Single-write.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderWorkspace {
    pub id: String,
    pub sort_order: u32,
}

impl Command<Ctx> for ReorderWorkspace {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = WorkspaceId::new(&self.id);
        let mut ws = WorkspaceRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::WorkspaceNotFound(self.id.clone()))?;
        ws.sort_order = self.sort_order;
        WorkspaceRepo::update(cx.db(), &ws).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::test_util::*;

    // Scenario: Reorder sets sort_order.
    #[tokio::test]
    async fn reorder_workspace_sets_sort_order() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-ord-1", "Orderable").await;
        ReorderWorkspace {
            id: "ws-ord-1".to_owned(),
            sort_order: 42,
        }
        .handle(&cx)
        .await
        .unwrap();
        let ws = WorkspaceRepo::get(cx.db(), &ws_id("ws-ord-1"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ws.sort_order, 42);
    }
}
