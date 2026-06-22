use crate::context::Ctx;
use crate::entities::{LaunchTemplate, ProjectId};
use crate::infra::LaunchTemplateRepo;
use crate::shared::cqs::Query;
use crate::shared::pagination::{Listing, Page};
use crate::shared::Result;

/// List a project's launch templates.
pub struct ListLaunchTemplatesByProject {
    pub project_id: ProjectId,
    pub page: Page,
}

impl Query<Ctx> for ListLaunchTemplatesByProject {
    type Out = Listing<LaunchTemplate>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        LaunchTemplateRepo::list(cx.db(), &self.project_id, &self.page).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;

    use super::super::new_launch_template_cmd::NewLaunchTemplateCmd;

    #[tokio::test]
    async fn list_launch_templates_filters_by_project() {
        let dir = tempfile::TempDir::new().unwrap();
        let (cx, bus) = ctx(&dir).await;

        let other_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO project (id, workspace_id, name, source_kind) VALUES (?, ?, ?, ?)",
        )
        .bind(&other_id)
        .bind("00000000-0000-0000-0000-000000000001")
        .bind("OtherProject")
        .bind("blank")
        .execute(cx.db())
        .await
        .unwrap();

        bus.execute(NewLaunchTemplateCmd {
            project_id: ProjectId::new(UNFILED),
            spec_version: 1,
            spec_json: "{}".to_owned(),
        })
        .await
        .unwrap();
        bus.execute(NewLaunchTemplateCmd {
            project_id: ProjectId::new(&other_id),
            spec_version: 1,
            spec_json: "{}".to_owned(),
        })
        .await
        .unwrap();

        let listing = bus
            .query(ListLaunchTemplatesByProject {
                project_id: ProjectId::new(UNFILED),
                page: Page::All,
            })
            .await
            .unwrap();

        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].project_id, ProjectId::new(UNFILED));
    }
}
