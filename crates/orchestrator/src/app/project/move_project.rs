use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::entities::workspace::WorkspaceId;
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Error, Result};

/// Reassign a project to another workspace.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveProject {
    pub id: String,
    pub workspace_id: String,
}

impl Command<Ctx> for MoveProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = ProjectId::new(&self.id);
        let mut project = ProjectRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.id.clone()))?;
        project.guard_not_unfiled()?;
        project.workspace_id = WorkspaceId::new(&self.workspace_id);
        ProjectRepo::update(cx.db(), &project).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    use super::super::list_projects_by_workspace::ListProjectsByWorkspace;

    #[tokio::test]
    async fn move_project_reparents_to_new_workspace() {
        let (ctx, bus) = ctx().await;

        let other_ws_id = "ws-move-test-0001";
        sqlx::query("INSERT INTO workspace (id, name) VALUES (?, ?)")
            .bind(other_ws_id)
            .bind("Other")
            .execute(ctx.db())
            .await
            .unwrap();

        seed_project(ctx.db(), "p-move", "Mover", &default_ws()).await;

        bus.execute(MoveProject {
            id: "p-move".to_owned(),
            workspace_id: other_ws_id.to_owned(),
        })
        .await
        .unwrap();

        // Must appear under the new workspace.
        let new_ws_listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: other_ws_id.to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert!(new_ws_listing.items.iter().any(|p| p.id == "p-move"));

        // Must not appear under the old workspace.
        let old_ws_listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws().as_str().to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert!(!old_ws_listing.items.iter().any(|p| p.id == "p-move"));
    }

    #[tokio::test]
    async fn move_unfiled_project_is_rejected() {
        let (_ctx, bus) = ctx().await;

        let other_ws_id = "ws-move-unfiled";
        // No need to seed workspace since error is returned before DB call.
        let result = bus
            .execute(MoveProject {
                id: unfiled_project_id().as_str().to_owned(),
                workspace_id: other_ws_id.to_owned(),
            })
            .await;
        assert!(
            matches!(result, Err(Error::ProjectIsUnfiled)),
            "unfiled project must not be moved: {result:?}"
        );
    }
}
