use crate::context::Ctx;
use crate::entities::project::Project;
use crate::entities::workspace::WorkspaceId;
use crate::infra::project::ProjectRepo;
use crate::shared::{Query, Result};

/// Fuzzy-search projects by name (sqlite-side LIKE; no app-side table scan).
pub struct SearchProjects {
    pub workspace_id: WorkspaceId,
    pub query: String,
    pub limit: u32,
}

impl Query<Ctx> for SearchProjects {
    type Out = Vec<Project>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        ProjectRepo::search(cx.db(), &self.workspace_id, &self.query, self.limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    #[tokio::test]
    async fn search_projects_returns_matching_projects() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-search-1", "Alpha Project", &default_ws()).await;
        seed_project(ctx.db(), "p-search-2", "Beta Project", &default_ws()).await;
        seed_project(ctx.db(), "p-search-3", "Gamma Thing", &default_ws()).await;

        let results = bus
            .query(SearchProjects {
                workspace_id: default_ws(),
                query: "Project".to_owned(),
                limit: 10,
            })
            .await
            .unwrap();

        let names: Vec<&str> = results.iter().map(|p| p.name.as_str()).collect();
        assert!(
            names.contains(&"Alpha Project"),
            "must contain Alpha Project"
        );
        assert!(names.contains(&"Beta Project"), "must contain Beta Project");
        assert!(
            !names.contains(&"Gamma Thing"),
            "must not contain non-matching project"
        );
    }

    #[tokio::test]
    async fn search_projects_is_case_insensitive() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-case", "MyProject", &default_ws()).await;

        let results = bus
            .query(SearchProjects {
                workspace_id: default_ws(),
                query: "myproject".to_owned(),
                limit: 10,
            })
            .await
            .unwrap();

        assert!(results.iter().any(|p| p.id.as_str() == "p-case"));
    }

    #[tokio::test]
    async fn search_projects_empty_results_for_no_match() {
        let (_ctx, bus) = ctx().await;

        let results = bus
            .query(SearchProjects {
                workspace_id: default_ws(),
                query: "xyzzyxyzzy".to_owned(),
                limit: 10,
            })
            .await
            .unwrap();

        assert!(results.is_empty());
    }
}
