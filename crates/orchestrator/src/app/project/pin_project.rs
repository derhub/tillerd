use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Error, Result};

/// Toggle the `pinned` flag on (pin the project).
pub struct PinProject {
    pub id: ProjectId,
}

impl Command<Ctx> for PinProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut project = ProjectRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.id.as_str().to_owned()))?;
        project.pinned = true;
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
    async fn pinned_project_sorts_before_unpinned() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-unpin", "Unpinned", &default_ws()).await;
        seed_project(ctx.db(), "p-pin", "Pinned", &default_ws()).await;

        bus.execute(PinProject {
            id: ProjectId::new("p-pin"),
        })
        .await
        .unwrap();

        let listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws(),
                page: Page::All,
            })
            .await
            .unwrap();

        let own: Vec<&str> = listing
            .items
            .iter()
            .filter(|p| p.id.as_str() == "p-pin" || p.id.as_str() == "p-unpin")
            .map(|p| p.id.as_str())
            .collect();

        assert_eq!(own, vec!["p-pin", "p-unpin"]);
    }
}
