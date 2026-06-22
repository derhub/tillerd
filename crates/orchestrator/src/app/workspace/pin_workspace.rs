use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::entities::workspace::WorkspaceId;
use crate::infra::WorkspaceRepo;
use crate::shared::cqs::Command;
use crate::shared::{Error, Result};

/// Pin a workspace (it sorts before unpinned ones).
#[derive(Debug, Serialize, Deserialize)]
pub struct PinWorkspace {
    pub id: WorkspaceId,
}

impl Command<Ctx> for PinWorkspace {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut ws = WorkspaceRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::WorkspaceNotFound(self.id.as_str().to_owned()))?;
        ws.pinned = true;
        WorkspaceRepo::update(cx.db(), &ws).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::get_workspace_by_id::GetWorkspaceById;
    use crate::app::workspace::list_workspaces::ListWorkspaces;
    use crate::app::workspace::test_util::*;
    use crate::shared::cqs::Query;
    use crate::shared::pagination::Page;

    // Scenario: Pin/Unpin toggles pinned flag.
    #[tokio::test]
    async fn pin_workspace_sets_pinned_flag() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-pin-1", "Pinnable").await;
        PinWorkspace {
            id: ws_id("ws-pin-1"),
        }
        .handle(&cx)
        .await
        .unwrap();
        let ws = GetWorkspaceById {
            id: ws_id("ws-pin-1"),
        }
        .handle(&cx)
        .await
        .unwrap()
        .unwrap();
        assert!(ws.pinned);
    }

    // Scenario: A pinned item sorts ahead of unpinned.
    #[tokio::test]
    async fn list_workspaces_returns_pinned_first() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-lst-unpinned", "Unpinned").await;
        insert_workspace(&cx, "ws-lst-pinned", "Pinned").await;
        PinWorkspace {
            id: ws_id("ws-lst-pinned"),
        }
        .handle(&cx)
        .await
        .unwrap();

        let listing = ListWorkspaces { page: Page::All }
            .handle(&cx)
            .await
            .unwrap();
        let first = listing.items.first().expect("at least one workspace");
        assert!(first.pinned, "pinned workspace must appear first");
    }
}
