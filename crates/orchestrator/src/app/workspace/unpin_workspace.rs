use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::entities::workspace::WorkspaceId;
use crate::infra::WorkspaceRepo;
use crate::shared::cqs::Command;
use crate::shared::{Error, Result};

/// Unpin a workspace.
#[derive(Debug, Serialize, Deserialize)]
pub struct UnpinWorkspace {
    pub id: WorkspaceId,
}

impl Command<Ctx> for UnpinWorkspace {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut ws = WorkspaceRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::WorkspaceNotFound(self.id.as_str().to_owned()))?;
        ws.pinned = false;
        WorkspaceRepo::update(cx.db(), &ws).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::get_workspace_by_id::GetWorkspaceById;
    use crate::app::workspace::pin_workspace::PinWorkspace;
    use crate::app::workspace::test_util::*;
    use crate::shared::cqs::Query;

    #[tokio::test]
    async fn unpin_workspace_clears_pinned_flag() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-unpin-1", "Pinnable").await;
        PinWorkspace {
            id: ws_id("ws-unpin-1"),
        }
        .handle(&cx)
        .await
        .unwrap();
        UnpinWorkspace {
            id: ws_id("ws-unpin-1"),
        }
        .handle(&cx)
        .await
        .unwrap();
        let ws = GetWorkspaceById {
            id: ws_id("ws-unpin-1"),
        }
        .handle(&cx)
        .await
        .unwrap()
        .unwrap();
        assert!(!ws.pinned);
    }
}
