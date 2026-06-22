use crate::context::Ctx;
use crate::entities::LaunchTemplateId;
use crate::infra::LaunchTemplateRepo;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Delete a project's launch template.
pub struct DiscardLaunchTemplate {
    pub id: LaunchTemplateId,
}

impl Command<Ctx> for DiscardLaunchTemplate {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        LaunchTemplateRepo::delete(cx.db(), &self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;
    use crate::entities::ProjectId;
    use crate::shared::pagination::Page;

    use super::super::get_launch_template_by_id::GetLaunchTemplateById;
    use super::super::list_launch_templates_by_project::ListLaunchTemplatesByProject;
    use super::super::new_launch_template_cmd::NewLaunchTemplateCmd;

    #[tokio::test]
    async fn discard_launch_template_removes_the_row() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        bus.execute(NewLaunchTemplateCmd {
            project_id: ProjectId::new(UNFILED),
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
        let id = listing.items[0].id.clone();

        bus.execute(DiscardLaunchTemplate { id: id.clone() })
            .await
            .unwrap();

        let got = bus
            .query(GetLaunchTemplateById { id })
            .await
            .unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn discard_launch_template_on_missing_id_returns_not_found() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        let err = bus
            .execute(DiscardLaunchTemplate {
                id: LaunchTemplateId::mint(),
            })
            .await
            .unwrap_err();

        assert_eq!(err.code(), "launch_template.not_found");
    }
}
