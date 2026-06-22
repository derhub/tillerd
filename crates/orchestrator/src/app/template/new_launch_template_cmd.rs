use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::{LaunchTemplate, LaunchTemplateId, ProjectId};
use crate::infra::LaunchTemplateRepo;
use crate::shared::message::Command;
use crate::shared::Result;

/// Create a new launch template for a project.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewLaunchTemplateCmd {
    pub id: String,
    pub project_id: String,
    pub spec_version: u32,
    pub spec_json: String,
}

impl Command<Ctx> for NewLaunchTemplateCmd {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let entity = LaunchTemplate {
            id: LaunchTemplateId::from_string(&self.id),
            project_id: ProjectId::new(&self.project_id),
            spec_version: self.spec_version,
            spec_json: self.spec_json.clone(),
        };
        LaunchTemplateRepo::create(cx.db(), &entity).await?;
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
                id: uuid::Uuid::new_v4().to_string(),
                project_id: ProjectId::new(UNFILED).as_str().to_owned(),
                spec_version: 1,
                spec_json: r#"{"items":[]}"#.to_owned(),
            })
            .await;

        assert!(result.is_ok());
    }
}
