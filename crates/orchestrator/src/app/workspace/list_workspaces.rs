use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::entities::workspace::Workspace;
use crate::infra::WorkspaceRepo;
use crate::shared::cqs::Query;
use crate::shared::pagination::{Listing, Page};
use crate::shared::Result;

/// List all workspaces, pinned-first then by sort order.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListWorkspaces {
    pub page: Page,
}

impl Query<Ctx> for ListWorkspaces {
    type Out = Listing<Workspace>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        WorkspaceRepo::list(cx.db(), &self.page).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::test_util::*;

    // Scenario: A query reads and does not mutate — ListWorkspaces.
    #[tokio::test]
    async fn list_workspaces_returns_all_workspaces() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-list-a", "A").await;
        insert_workspace(&cx, "ws-list-b", "B").await;
        let listing = ListWorkspaces { page: Page::All }
            .handle(&cx)
            .await
            .unwrap();
        let ids: Vec<&str> = listing.items.iter().map(|w| w.id.as_str()).collect();
        assert!(ids.contains(&"ws-list-a"));
        assert!(ids.contains(&"ws-list-b"));
    }
}
