use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Error, Result};

/// Set a project's sort order.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderProject {
    pub id: String,
    pub sort_order: u32,
}

impl Command<Ctx> for ReorderProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = ProjectId::new(&self.id);
        let mut project = ProjectRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.id.clone()))?;
        project.sort_order = self.sort_order;
        ProjectRepo::update(cx.db(), &project).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    #[tokio::test]
    async fn reorder_project_sets_sort_order() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-reorder", "Reorderable", &default_ws()).await;

        bus.execute(ReorderProject {
            id: "p-reorder".to_owned(),
            sort_order: 42,
        })
        .await
        .unwrap();

        let project = ProjectRepo::get(ctx.db(), &ProjectId::new("p-reorder"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(project.sort_order, 42);
    }
}
