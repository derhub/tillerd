use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::entities::workspace::WorkspaceId;
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Error, Result};

/// Reassign a project to another workspace.
pub struct MoveProject {
    pub id: ProjectId,
    pub workspace_id: WorkspaceId,
}

impl Command<Ctx> for MoveProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut project = ProjectRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.id.as_str().to_owned()))?;
        project.guard_not_unfiled()?;
        project.workspace_id = self.workspace_id.clone();
        ProjectRepo::update(cx.db(), &project).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;
    use crate::shared::pagination::Page;

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
            id: ProjectId::new("p-move"),
            workspace_id: WorkspaceId::new(other_ws_id),
        })
        .await
        .unwrap();

        // Must appear under the new workspace.
        let new_ws_listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: WorkspaceId::new(other_ws_id),
                page: Page::All,
            })
            .await
            .unwrap();
        assert!(new_ws_listing
            .items
            .iter()
            .any(|p| p.id.as_str() == "p-move"));

        // Must not appear under the old workspace.
        let old_ws_listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws(),
                page: Page::All,
            })
            .await
            .unwrap();
        assert!(!old_ws_listing
            .items
            .iter()
            .any(|p| p.id.as_str() == "p-move"));
    }

    #[tokio::test]
    async fn move_unfiled_project_is_rejected() {
        let (_ctx, bus) = ctx().await;

        let other_ws_id = "ws-move-unfiled";
        // No need to seed workspace since error is returned before DB call.
        let result = bus
            .execute(MoveProject {
                id: unfiled_project_id(),
                workspace_id: WorkspaceId::new(other_ws_id),
            })
            .await;
        assert!(
            matches!(result, Err(Error::ProjectIsUnfiled)),
            "unfiled project must not be moved: {result:?}"
        );
    }
}
