use crate::context::Ctx;
use crate::entities::project::Project;
use crate::entities::workspace::WorkspaceId;
use crate::infra::project::ProjectRepo;
use crate::shared::pagination::{Listing, Page};
use crate::shared::{Query, Result};

/// List projects in a workspace (pinned-first then by sort_order).
pub struct ListProjectsByWorkspace {
    pub workspace_id: WorkspaceId,
    pub page: Page,
}

impl Query<Ctx> for ListProjectsByWorkspace {
    type Out = Listing<Project>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        ProjectRepo::list(cx.db(), &self.workspace_id, &self.page).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    #[tokio::test]
    async fn list_projects_by_workspace_does_not_mutate() {
        let (_ctx, bus) = ctx().await;
        // Query returns listing; no write occurs.
        let listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws(),
                page: Page::All,
            })
            .await
            .unwrap();
        // The seed Unfiled project is always present.
        assert!(!listing.items.is_empty());
    }

    #[tokio::test]
    async fn list_projects_by_workspace_filters_correctly() {
        let (ctx, bus) = ctx().await;

        // Seed a second workspace.
        let other_ws_id = "ws-other-test-0001";
        sqlx::query("INSERT INTO workspace (id, name) VALUES (?, ?)")
            .bind(other_ws_id)
            .bind("Other")
            .execute(ctx.db())
            .await
            .unwrap();

        seed_project(ctx.db(), "p-ws1", "In Default", &default_ws()).await;
        seed_project(
            ctx.db(),
            "p-ws2",
            "In Other",
            &WorkspaceId::new(other_ws_id),
        )
        .await;

        let listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws(),
                page: Page::All,
            })
            .await
            .unwrap();

        let ids: Vec<&str> = listing.items.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"p-ws1"), "own project must appear");
        assert!(!ids.contains(&"p-ws2"), "other-ws project must not appear");
    }
}
