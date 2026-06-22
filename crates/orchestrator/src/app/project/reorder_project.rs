use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Error, Result};

/// Set a project's sort order.
pub struct ReorderProject {
    pub id: ProjectId,
    pub sort_order: u32,
}

impl Command<Ctx> for ReorderProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut project = ProjectRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.id.as_str().to_owned()))?;
        project.sort_order = self.sort_order;
        ProjectRepo::update(cx.db(), &project).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    use super::super::get_project_by_id::GetProjectById;

    #[tokio::test]
    async fn reorder_project_sets_sort_order() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-reorder", "Reorderable", &default_ws()).await;

        bus.execute(ReorderProject {
            id: ProjectId::new("p-reorder"),
            sort_order: 42,
        })
        .await
        .unwrap();

        let project = bus
            .query(GetProjectById {
                id: ProjectId::new("p-reorder"),
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(project.sort_order, 42);
    }
}
