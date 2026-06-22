use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::workspace::WorkspaceId;
use crate::infra::WorkspaceRepo;
use crate::shared::message::Command;
use crate::shared::{Error, Result};

/// Pin a workspace (it sorts before unpinned ones).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinWorkspace {
    pub id: String,
}

impl Command<Ctx> for PinWorkspace {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = WorkspaceId::new(&self.id);
        let mut ws = WorkspaceRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::WorkspaceNotFound(self.id.clone()))?;
        ws.pinned = true;
        WorkspaceRepo::update(cx.db(), &ws).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::list_workspaces::ListWorkspaces;
    use crate::app::workspace::test_util::*;
    use crate::shared::message::Query;

    // Scenario: Pin/Unpin toggles pinned flag.
    #[tokio::test]
    async fn pin_workspace_sets_pinned_flag() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-pin-1", "Pinnable").await;
        PinWorkspace {
            id: "ws-pin-1".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();
        let ws = WorkspaceRepo::get(cx.db(), &ws_id("ws-pin-1"))
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
            id: "ws-lst-pinned".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();

        let listing = ListWorkspaces {
            limit: None,
            offset: None,
            after: None,
        }
        .handle(&cx)
        .await
        .unwrap();
        let first = listing.items.first().expect("at least one workspace");
        assert_eq!(
            first.id, "ws-lst-pinned",
            "pinned workspace must appear first"
        );
    }
}
