use crate::context::Ctx;
use crate::entities::project::{Project, ProjectId};
use crate::infra::project::ProjectRepo;
use crate::shared::{Query, Result};

/// Fetch one project by id.
pub struct GetProjectById {
    pub id: ProjectId,
}

impl Query<Ctx> for GetProjectById {
    type Out = Option<Project>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        ProjectRepo::get(cx.db(), &self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    #[tokio::test]
    async fn get_project_by_id_returns_the_project() {
        let (ctx, bus) = ctx().await;
        let project = seed_project(ctx.db(), "p-get", "Gamma", &default_ws()).await;

        let fetched = bus
            .query(GetProjectById {
                id: project.id.clone(),
            })
            .await
            .unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "Gamma");
    }

    #[tokio::test]
    async fn get_project_by_id_returns_none_for_absent() {
        let (_ctx, bus) = ctx().await;
        let result = bus
            .query(GetProjectById {
                id: ProjectId::new("no-such-id"),
            })
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
