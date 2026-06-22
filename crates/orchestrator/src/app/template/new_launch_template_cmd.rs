use crate::context::Ctx;
use crate::entities::{NewLaunchTemplate, ProjectId};
use crate::infra::LaunchTemplateRepo;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Create a new launch template for a project.
pub struct NewLaunchTemplateCmd {
    pub project_id: ProjectId,
    pub spec_version: u32,
    pub spec_json: String,
}

impl Command<Ctx> for NewLaunchTemplateCmd {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let draft = NewLaunchTemplate {
            project_id: self.project_id.clone(),
            spec_version: self.spec_version,
            spec_json: self.spec_json.clone(),
        };
        LaunchTemplateRepo::create(cx.db(), &draft).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;

    #[tokio::test]
    async fn new_launch_template_creates_a_row_and_returns_nothing() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;

        let result = bus
            .execute(NewLaunchTemplateCmd {
                project_id: ProjectId::new(UNFILED),
                spec_version: 1,
                spec_json: r#"{"items":[]}"#.to_owned(),
            })
            .await;

        assert!(result.is_ok());
    }
}
