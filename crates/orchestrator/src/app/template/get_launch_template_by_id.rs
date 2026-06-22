use crate::context::Ctx;
use crate::entities::{LaunchTemplate, LaunchTemplateId, ProjectId};
use crate::infra::LaunchTemplateRepo;
use crate::shared::cqs::Query;
use crate::shared::pagination::Page;
use crate::shared::Result;

/// Fetch one launch template by id.
pub struct GetLaunchTemplateById {
    pub id: LaunchTemplateId,
}

impl Query<Ctx> for GetLaunchTemplateById {
    type Out = Option<LaunchTemplate>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        LaunchTemplateRepo::get(cx.db(), &self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;

    use super::super::list_launch_templates_by_project::ListLaunchTemplatesByProject;
    use super::super::new_launch_template_cmd::NewLaunchTemplateCmd;

    #[tokio::test]
    async fn get_launch_template_by_id_returns_created_template() {
        let dir = tempfile::TempDir::new().unwrap();
        let (cx, bus) = ctx(&dir).await;

        bus.execute(NewLaunchTemplateCmd {
            project_id: ProjectId::new(UNFILED),
            spec_version: 1,
            spec_json: r#"{"items":[]}"#.to_owned(),
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
        let id = listing.items[0].id.clone();

        let got = bus
            .query(GetLaunchTemplateById { id: id.clone() })
            .await
            .unwrap();

        assert!(got.is_some());
        let tmpl = got.unwrap();
        assert_eq!(tmpl.id, id);
        assert_eq!(tmpl.spec_version, 1);
        assert_eq!(tmpl.spec_json, r#"{"items":[]}"#);

        // query wrote nothing — count still 1
        let listing2 = bus
            .query(ListLaunchTemplatesByProject {
                project_id: ProjectId::new(UNFILED),
                page: Page::All,
            })
            .await
            .unwrap();
        assert_eq!(listing2.items.len(), 1);
        let _ = cx;
    }
}
