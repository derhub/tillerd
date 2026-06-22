use serde::Deserialize;

use crate::app::project::ProjectView;
use crate::context::Ctx;
use crate::shared::{Query, Result};

/// Fetch one project by id.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProjectById {
    pub id: String,
}

impl Query<Ctx> for GetProjectById {
    type Out = Option<ProjectView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        Ok(sqlx::query_as::<_, ProjectView>(
            "SELECT id, name, source_kind, root_path, workspace_id FROM project WHERE id = ?",
        )
        .bind(&self.id)
        .fetch_optional(cx.db())
        .await?)
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
                id: project.id.as_str().to_owned(),
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
                id: "no-such-id".to_owned(),
            })
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
