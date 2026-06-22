use crate::context::Ctx;
use crate::entities::{LaunchTemplate, LaunchTemplateId, ProjectId};
use crate::infra::LaunchTemplateRepo;
use crate::shared::cqs::Command;
use crate::shared::{Error, Result};

/// Replace the spec on an existing launch template.
pub struct ApplyTemplateSpec {
    pub id: LaunchTemplateId,
    pub spec_version: u32,
    pub spec_json: String,
}

impl Command<Ctx> for ApplyTemplateSpec {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let existing = LaunchTemplateRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::LaunchTemplateNotFound(self.id.as_str().to_owned()))?;
        let updated = LaunchTemplate {
            spec_version: self.spec_version,
            spec_json: self.spec_json.clone(),
            ..existing
        };
        LaunchTemplateRepo::update(cx.db(), &updated).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;
    use crate::shared::pagination::Page;

    use super::super::get_launch_template_by_id::GetLaunchTemplateById;
    use super::super::list_launch_templates_by_project::ListLaunchTemplatesByProject;
    use super::super::new_launch_template_cmd::NewLaunchTemplateCmd;

    #[tokio::test]
    async fn apply_template_spec_replaces_the_saved_spec() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

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
        let id = listing.items[0].id.clone();

        bus.execute(ApplyTemplateSpec {
            id: id.clone(),
            spec_version: 2,
            spec_json: r#"{"items":["a"]}"#.to_owned(),
        })
        .await
        .unwrap();

        let updated = bus
            .query(GetLaunchTemplateById { id })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.spec_version, 2);
        assert_eq!(updated.spec_json, r#"{"items":["a"]}"#);
    }

    #[tokio::test]
    async fn apply_template_spec_on_missing_id_returns_not_found() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        let err = bus
            .execute(ApplyTemplateSpec {
                id: LaunchTemplateId::mint(),
                spec_version: 1,
                spec_json: "{}".to_owned(),
            })
            .await
            .unwrap_err();

        assert_eq!(err.code(), "launch_template.not_found");
    }
}
