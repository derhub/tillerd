use serde::Deserialize;

use crate::app::project::ProjectView;
use crate::context::Ctx;
use crate::shared::{Query, Result};

/// Fuzzy-search projects by name (sqlite-side LIKE; no app-side table scan).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchProjects {
    pub workspace_id: String,
    pub query: String,
    pub limit: u32,
}

impl Query<Ctx> for SearchProjects {
    type Out = Vec<ProjectView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let pattern = format!("%{}%", self.query.replace('%', "\\%").replace('_', "\\_"));
        Ok(sqlx::query_as::<_, ProjectView>(
            "SELECT id, name, source_kind, root_path, workspace_id, pinned,
                    CASE WHEN archived_at IS NOT NULL THEN 'archived' ELSE 'active' END AS status
             FROM project
             WHERE workspace_id = ?
               AND name LIKE ? ESCAPE '\\'
             ORDER BY
               CASE WHEN lower(name) = lower(?) THEN 0
                    WHEN lower(name) LIKE lower(?) || '%' THEN 1
                    ELSE 2
               END,
               sort_order,
               id
             LIMIT ?",
        )
        .bind(&self.workspace_id)
        .bind(&pattern)
        .bind(&self.query)
        .bind(format!("{}%", self.query))
        .bind(self.limit as i64)
        .fetch_all(cx.db())
        .await?)
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
                workspace_id: default_ws().as_str().to_owned(),
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
                workspace_id: default_ws().as_str().to_owned(),
                query: "myproject".to_owned(),
                limit: 10,
            })
            .await
            .unwrap();

        assert!(results.iter().any(|p| p.id == "p-case"));
    }

    #[tokio::test]
    async fn search_projects_empty_results_for_no_match() {
        let (_ctx, bus) = ctx().await;

        let results = bus
            .query(SearchProjects {
                workspace_id: default_ws().as_str().to_owned(),
                query: "xyzzyxyzzy".to_owned(),
                limit: 10,
            })
            .await
            .unwrap();

        assert!(results.is_empty());
    }
}
