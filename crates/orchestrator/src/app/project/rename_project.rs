use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Error, Result};

/// Rename a project (trims whitespace; entity enforces no-mutation on Unfiled).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameProject {
    pub id: String,
    pub name: String,
}

impl Command<Ctx> for RenameProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = ProjectId::new(&self.id);
        let mut project = ProjectRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.id.clone()))?;
        project.rename(&self.name);
        ProjectRepo::update(cx.db(), &project).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    use super::super::get_project_by_id::GetProjectById;

    #[tokio::test]
    async fn rename_project_updates_name() {
        let (ctx, bus) = ctx().await;
        let project = seed_project(ctx.db(), "p-rename", "Old", &default_ws()).await;

        bus.execute(RenameProject {
            id: project.id.as_str().to_owned(),
            name: "New".to_owned(),
        })
        .await
        .unwrap();

        let fetched = bus
            .query(GetProjectById {
                id: project.id.as_str().to_owned(),
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.name, "New");
    }

    #[tokio::test]
    async fn rename_project_returns_nothing() {
        let (ctx, bus) = ctx().await;
        let project = seed_project(ctx.db(), "p-rename-2", "Foo", &default_ws()).await;

        let result = bus
            .execute(RenameProject {
                id: project.id.as_str().to_owned(),
                name: "Bar".to_owned(),
            })
            .await;
        // Command returns Result<()>.
        assert!(result.is_ok());
    }
}
