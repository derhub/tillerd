use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::LaunchTemplateId;
use crate::infra::LaunchTemplateRepo;
use crate::shared::message::Command;
use crate::shared::Result;

/// Delete a project's launch template.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardLaunchTemplate {
    pub id: String,
}

impl Command<Ctx> for DiscardLaunchTemplate {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = LaunchTemplateId::from_string(&self.id);
        LaunchTemplateRepo::delete(cx.db(), &id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;

    use super::super::get_launch_template_by_id::GetLaunchTemplateById;
    use super::super::list_launch_templates_by_project::ListLaunchTemplatesByProject;
    use super::super::new_launch_template_cmd::NewLaunchTemplateCmd;

    #[tokio::test]
    async fn discard_launch_template_removes_the_row() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        bus.execute(NewLaunchTemplateCmd {
            project_id: UNFILED.to_owned(),
            spec_version: 1,
            spec_json: "{}".to_owned(),
        })
        .await
        .unwrap();

        let listing = bus
            .query(ListLaunchTemplatesByProject {
                project_id: UNFILED.to_owned(),
                limit: None,
                offset: None,
                after: None,
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
                id: LaunchTemplateId::mint().as_str().to_owned(),
            })
            .await
            .unwrap_err();

        assert_eq!(err.code(), "launch_template.not_found");
    }
}
