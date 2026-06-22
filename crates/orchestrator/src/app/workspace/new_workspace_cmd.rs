use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::workspace::{Workspace, WorkspaceId, WorkspaceStatus};
use crate::infra::WorkspaceRepo;
use crate::shared::message::Command;
use crate::shared::Result;

/// Create a workspace with the given id and name.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewWorkspaceCmd {
    pub id: String,
    pub name: String,
}

impl Command<Ctx> for NewWorkspaceCmd {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let workspace = Workspace {
            id: WorkspaceId::new(&self.id),
            name: self.name.clone(),
            sort_order: 0,
            pinned: false,
            status: WorkspaceStatus::Active,
        };
        WorkspaceRepo::create(cx.db(), &workspace).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::test_util::*;
    use crate::infra::WorkspaceRepo;

    // Scenario: A command mutates and returns nothing.
    #[tokio::test]
    async fn new_workspace_creates_and_returns_unit() {
        let cx = ctx().await;
        let result = NewWorkspaceCmd {
            id: "ws-new-1".to_owned(),
            name: "Alpha".to_owned(),
        }
        .handle(&cx)
        .await;
        assert!(result.is_ok());

        let ws = WorkspaceRepo::get(cx.db(), &ws_id("ws-new-1"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ws.name, "Alpha");
    }
}
