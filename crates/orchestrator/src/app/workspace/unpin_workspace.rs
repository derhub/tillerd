use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::workspace::WorkspaceId;
use crate::infra::WorkspaceRepo;
use crate::shared::message::Command;
use crate::shared::{Error, Result};

/// Unpin a workspace.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnpinWorkspace {
    pub id: String,
}

impl Command<Ctx> for UnpinWorkspace {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = WorkspaceId::new(&self.id);
        let mut ws = WorkspaceRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::WorkspaceNotFound(self.id.clone()))?;
        ws.pinned = false;
        WorkspaceRepo::update(cx.db(), &ws).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::pin_workspace::PinWorkspace;
    use crate::app::workspace::test_util::*;

    #[tokio::test]
    async fn unpin_workspace_clears_pinned_flag() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-unpin-1", "Pinnable").await;
        PinWorkspace {
            id: "ws-unpin-1".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();
        UnpinWorkspace {
            id: "ws-unpin-1".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();
        let ws = WorkspaceRepo::get(cx.db(), &ws_id("ws-unpin-1"))
            .await
            .unwrap()
            .unwrap();
        assert!(!ws.pinned);
    }
}
