use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::entities::workspace::{NewWorkspace, WorkspaceId};
use crate::infra::WorkspaceRepo;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Create a workspace with the given name.
#[derive(Debug, Serialize, Deserialize)]
pub struct NewWorkspaceCmd {
    pub id: WorkspaceId,
    pub name: String,
}

impl Command<Ctx> for NewWorkspaceCmd {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        WorkspaceRepo::create(
            cx.db(),
            &NewWorkspace {
                name: self.name.clone(),
            },
            &self.id,
        )
        .await
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
        let id = ws_id("ws-new-1");
        let result = NewWorkspaceCmd {
            id: id.clone(),
            name: "Alpha".to_owned(),
        }
        .handle(&cx)
        .await;
        assert!(result.is_ok());

        let ws = WorkspaceRepo::get(cx.db(), &id).await.unwrap().unwrap();
        assert_eq!(ws.name, "Alpha");
    }
}
