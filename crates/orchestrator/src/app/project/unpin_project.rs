use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Error, Result};

/// Toggle the `pinned` flag off (unpin the project).
pub struct UnpinProject {
    pub id: ProjectId,
}

impl Command<Ctx> for UnpinProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut project = ProjectRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.id.as_str().to_owned()))?;
        project.pinned = false;
        ProjectRepo::update(cx.db(), &project).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    use super::super::get_project_by_id::GetProjectById;
    use super::super::pin_project::PinProject;

    #[tokio::test]
    async fn unpin_project_moves_it_after_pinned() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-a", "A", &default_ws()).await;
        seed_project(ctx.db(), "p-b", "B", &default_ws()).await;

        bus.execute(PinProject {
            id: ProjectId::new("p-a"),
        })
        .await
        .unwrap();
        bus.execute(UnpinProject {
            id: ProjectId::new("p-a"),
        })
        .await
        .unwrap();

        let project = bus
            .query(GetProjectById {
                id: ProjectId::new("p-a"),
            })
            .await
            .unwrap()
            .unwrap();
        assert!(!project.pinned);
    }
}
