use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::entities::session::{Session, SessionId, SessionStatus, TitleSource};
use crate::infra::session::SessionRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;

use super::common::now_iso;

/// Create a session in a project. When `template_id` is set the session's launch
/// spec is copied from the template; otherwise spec is empty.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSessionCmd {
    pub id: SessionId,
    pub project_id: Option<String>,
    pub title_source: String,
    /// Required when `title_source == custom`; used as branch/agent-title otherwise.
    pub title: Option<String>,
    /// When set, the session's spec blob and version are copied from this template.
    pub template_id: Option<String>,
}

impl Command<Ctx> for NewSessionCmd {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let project_id = self
            .project_id
            .clone()
            .map(ProjectId::new)
            .ok_or(Error::Validation {
                field: "project_id",
                reason: "required".to_owned(),
            })?;

        let title_source = match self.title_source.as_str() {
            "branch" => TitleSource::Branch,
            "both" => TitleSource::Both,
            "custom" => TitleSource::Custom,
            _ => TitleSource::AgentTitle,
        };

        let title = self.title.clone().unwrap_or_default().trim().to_owned();

        let (spec_version, spec_json) = match &self.template_id {
            Some(tid) => {
                use crate::entities::launch_template::LaunchTemplateId;
                use crate::infra::LaunchTemplateRepo;
                let ltid = LaunchTemplateId::from_string(tid);
                let tmpl = LaunchTemplateRepo::get(cx.db(), &ltid)
                    .await?
                    .ok_or_else(|| Error::LaunchTemplateNotFound(tid.clone()))?;
                (Some(tmpl.spec_version), Some(tmpl.spec_json))
            }
            None => (None, None),
        };

        let session = Session {
            id: self.id.clone(),
            project_id,
            title,
            title_source,
            created_at: now_iso(),
            spec_version,
            spec_json,
            sort_order: 0,
            pinned: false,
            status: SessionStatus::Active,
        };
        SessionRepo::create(cx.db(), &session).await
    }
}

#[cfg(test)]
mod tests {
    use crate::app::session::list_sessions_by_project::ListSessionsByProject;
    use crate::app::session::test_util::{ctx, draft_cmd, unfiled};

    // Scenario: A command mutates and returns nothing
    #[tokio::test]
    async fn new_session_creates_and_list_returns_it() {
        let (bus, _) = ctx().await;
        bus.execute(draft_cmd(unfiled())).await.unwrap();
        let listing = bus
            .query(ListSessionsByProject {
                project_id: unfiled().as_str().to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].title, "My session");
    }
}
