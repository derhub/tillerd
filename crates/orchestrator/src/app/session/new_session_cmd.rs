use crate::context::Ctx;
use crate::entities::session::{NewSession, Session, SessionId, SessionStatus};
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::{Error, Result};

use super::common::now_iso;

/// Create a session in a project. When `draft.template_id` is set the session's
/// launch spec is copied from the template; otherwise spec is empty.
pub struct NewSessionCmd(pub NewSession);

impl Command<Ctx> for NewSessionCmd {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let draft = &self.0;
        let project_id = draft.project_id.clone().ok_or(Error::Validation {
            field: "project_id",
            reason: "required".to_owned(),
        })?;

        let title = draft.title.clone().unwrap_or_default().trim().to_owned();

        let (spec_version, spec_json) = match &draft.template_id {
            Some(tid) => {
                use crate::entities::launch_template::LaunchTemplateId;
                use crate::infra::LaunchTemplateRepo;
                let ltid = LaunchTemplateId::from_string(tid.as_str());
                let tmpl = LaunchTemplateRepo::get(cx.db(), &ltid)
                    .await?
                    .ok_or_else(|| Error::LaunchTemplateNotFound(tid.as_str().to_owned()))?;
                (Some(tmpl.spec_version), Some(tmpl.spec_json))
            }
            None => (None, None),
        };

        let session = Session {
            id: SessionId::mint(),
            project_id,
            title,
            title_source: draft.title_source,
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
    use super::*;
    use crate::app::session::list_sessions_by_project::ListSessionsByProject;
    use crate::app::session::test_util::{ctx, draft, unfiled};

    use crate::shared::pagination::Page;

    // Scenario: A command mutates and returns nothing
    #[tokio::test]
    async fn new_session_creates_and_list_returns_it() {
        let (bus, _) = ctx().await;
        bus.execute(NewSessionCmd(draft(unfiled()))).await.unwrap();
        let listing = bus
            .query(ListSessionsByProject {
                project_id: unfiled(),
                page: Page::All,
            })
            .await
            .unwrap();
        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].title, "My session");
    }
}
