use crate::context::Ctx;
use crate::entities::project::{ProjectId, ProjectStatus};
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Error, Result};

/// Restore an archived project to active state.
pub struct RestoreProject {
    pub id: ProjectId,
}

impl Command<Ctx> for RestoreProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut project = ProjectRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.id.as_str().to_owned()))?;
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
    use super::super::get_project_by_id::GetProjectById;

    #[tokio::test]
    async fn restore_project_makes_it_active_again() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-restore", "Restorable", &default_ws()).await;

        bus.execute(ArchiveProject {
            id: ProjectId::new("p-restore"),
        })
        .await
        .unwrap();

        bus.execute(RestoreProject {
            id: ProjectId::new("p-restore"),
        })
        .await
        .unwrap();

        let project = bus
            .query(GetProjectById {
                id: ProjectId::new("p-restore"),
            })
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
                id: ProjectId::new("p-restore-active"),
            })
            .await;
        assert!(
            matches!(result, Err(Error::ProjectNotArchived)),
            "restoring active project must be rejected: {result:?}"
        );
    }
}
