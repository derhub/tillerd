use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::workspace::WorkspaceId;
use crate::infra::WorkspaceRepo;
use crate::shared::message::Command;
use crate::shared::{Error, Result};

/// Rename a workspace. Trims whitespace; single-write.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameWorkspace {
    pub id: String,
    pub name: String,
}

impl Command<Ctx> for RenameWorkspace {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = WorkspaceId::new(&self.id);
        let mut ws = WorkspaceRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::WorkspaceNotFound(self.id.clone()))?;
        ws.rename(&self.name);
        WorkspaceRepo::update(cx.db(), &ws).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::test_util::*;

    // Scenario: Rename mutates, returns nothing.
    #[tokio::test]
    async fn rename_workspace_updates_name() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-ren-1", "Before").await;
        RenameWorkspace {
            id: "ws-ren-1".to_owned(),
            name: "After".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();
        let ws = WorkspaceRepo::get(cx.db(), &ws_id("ws-ren-1"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ws.name, "After");
    }

    #[tokio::test]
    async fn rename_workspace_trims_whitespace() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-ren-2", "Old").await;
        RenameWorkspace {
            id: "ws-ren-2".to_owned(),
            name: "  Trimmed  ".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();
        let ws = WorkspaceRepo::get(cx.db(), &ws_id("ws-ren-2"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ws.name, "Trimmed");
    }

    #[tokio::test]
    async fn rename_workspace_returns_err_for_missing_workspace() {
        let cx = ctx().await;
        let err = RenameWorkspace {
            id: "no-such".to_owned(),
            name: "X".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap_err();
        assert_eq!(err.code(), "workspace.not_found");
    }
}
