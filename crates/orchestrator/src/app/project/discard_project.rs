use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Error, Result};

use super::stop_project_surfaces::StopProjectSurfaces;

/// Hard-delete a project (Unfiled rejected). A forceful remove: stops any running
/// surfaces so nothing is orphaned, then deletes the project -- `ON DELETE CASCADE`
/// drops its sessions, surfaces, and launch templates. Unlike `ArchiveProject`, this
/// does not require the project to be idle; deleting tears the work down.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardProject {
    pub id: String,
}

impl Command<Ctx> for DiscardProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = ProjectId::new(&self.id);
        let project = ProjectRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.id.clone()))?;
        project.guard_not_unfiled()?;
        StopProjectSurfaces {
            id: self.id.clone(),
        }
        .handle(cx)
        .await?;
        ProjectRepo::delete(cx.db(), &id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    use super::super::archive_project::ArchiveProject;
    use super::super::get_project_by_id::GetProjectById;

    #[tokio::test]
    async fn discard_project_removes_it() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-discard", "Doomed", &default_ws()).await;

        bus.execute(ArchiveProject {
            id: "p-discard".to_owned(),
        })
        .await
        .unwrap();
        bus.execute(DiscardProject {
            id: "p-discard".to_owned(),
        })
        .await
        .unwrap();

        let result = bus
            .query(GetProjectById {
                id: "p-discard".to_owned(),
            })
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn discard_active_project_removes_it_without_requiring_archive() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-discard-active", "Active", &default_ws()).await;

        // An active project is force-removed in one command (no archive precondition).
        bus.execute(DiscardProject {
            id: "p-discard-active".to_owned(),
        })
        .await
        .unwrap();

        let result = bus
            .query(GetProjectById {
                id: "p-discard-active".to_owned(),
            })
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn discard_unfiled_project_is_rejected() {
        let (_ctx, bus) = ctx().await;
        let result = bus
            .execute(DiscardProject {
                id: unfiled_project_id().as_str().to_owned(),
            })
            .await;
        assert!(
            matches!(result, Err(Error::ProjectIsUnfiled)),
            "unfiled must not be discarded: {result:?}"
        );
    }
}
