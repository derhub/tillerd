use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Error, Result};

/// Toggle the `pinned` flag on (pin the project).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinProject {
    pub id: String,
}

impl Command<Ctx> for PinProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = ProjectId::new(&self.id);
        let mut project = ProjectRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.id.clone()))?;
        project.pinned = true;
        ProjectRepo::update(cx.db(), &project).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    use super::super::list_projects_by_workspace::ListProjectsByWorkspace;

    #[tokio::test]
    async fn pinned_project_sorts_before_unpinned() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-unpin", "Unpinned", &default_ws()).await;
        seed_project(ctx.db(), "p-pin", "Pinned", &default_ws()).await;

        bus.execute(PinProject {
            id: "p-pin".to_owned(),
        })
        .await
        .unwrap();

        let listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws().as_str().to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();

        let own: Vec<&str> = listing
            .items
            .iter()
            .filter(|p| p.id == "p-pin" || p.id == "p-unpin")
            .map(|p| p.id.as_str())
            .collect();

        assert_eq!(own, vec!["p-pin", "p-unpin"]);
    }
}
