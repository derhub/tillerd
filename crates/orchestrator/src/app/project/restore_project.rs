use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::project::{ProjectId, ProjectStatus};
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Error, Result};

/// Restore an archived project to active state.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreProject {
    pub id: String,
}

impl Command<Ctx> for RestoreProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = ProjectId::new(&self.id);
        let mut project = ProjectRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.id.clone()))?;
        project.guard_archived()?;
        project.status = ProjectStatus::Active;
        ProjectRepo::update(cx.db(), &project).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    use super::super::archive_project::ArchiveProject;

    #[tokio::test]
    async fn restore_project_makes_it_active_again() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-restore", "Restorable", &default_ws()).await;

        bus.execute(ArchiveProject {
            id: "p-restore".to_owned(),
        })
        .await
        .unwrap();

        bus.execute(RestoreProject {
            id: "p-restore".to_owned(),
        })
        .await
        .unwrap();

        let project = ProjectRepo::get(ctx.db(), &ProjectId::new("p-restore"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(project.status, ProjectStatus::Active);
    }

    #[tokio::test]
    async fn restore_active_project_is_rejected() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-restore-active", "Active", &default_ws()).await;

        let result = bus
            .execute(RestoreProject {
                id: "p-restore-active".to_owned(),
            })
            .await;
        assert!(
            matches!(result, Err(Error::ProjectNotArchived)),
            "restoring active project must be rejected: {result:?}"
        );
    }
}
