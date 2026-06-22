use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::entities::workspace::WorkspaceId;
use crate::infra::WorkspaceRepo;
use crate::shared::cqs::Command;
use crate::shared::{Error, Result};

/// Rename a workspace. Trims whitespace; single-write.
#[derive(Debug, Serialize, Deserialize)]
pub struct RenameWorkspace {
    pub id: WorkspaceId,
    pub name: String,
}

impl Command<Ctx> for RenameWorkspace {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut ws = WorkspaceRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::WorkspaceNotFound(self.id.as_str().to_owned()))?;
        ws.rename(&self.name);
        WorkspaceRepo::update(cx.db(), &ws).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::get_workspace_by_id::GetWorkspaceById;
    use crate::app::workspace::test_util::*;
    use crate::shared::cqs::Query;

    // Scenario: Rename mutates, returns nothing.
    #[tokio::test]
    async fn rename_workspace_updates_name() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-ren-1", "Before").await;
        RenameWorkspace {
            id: ws_id("ws-ren-1"),
            name: "After".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();
        let ws = GetWorkspaceById {
            id: ws_id("ws-ren-1"),
        }
        .handle(&cx)
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
            id: ws_id("ws-ren-2"),
            name: "  Trimmed  ".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();
        let ws = GetWorkspaceById {
            id: ws_id("ws-ren-2"),
        }
        .handle(&cx)
        .await
        .unwrap()
        .unwrap();
        assert_eq!(ws.name, "Trimmed");
    }

    #[tokio::test]
    async fn rename_workspace_returns_err_for_missing_workspace() {
        let cx = ctx().await;
        let err = RenameWorkspace {
            id: ws_id("no-such"),
            name: "X".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap_err();
        assert_eq!(err.code(), "workspace.not_found");
    }
}
