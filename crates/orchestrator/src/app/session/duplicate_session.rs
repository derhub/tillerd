use crate::context::Ctx;
use crate::entities::session::{Session, SessionId, SessionStatus, TitleSource};
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::{Error, Result};

use super::common::now_iso;

/// Clone a session with its launch spec into the same project.
pub struct DuplicateSession {
    pub id: SessionId,
}

impl Command<Ctx> for DuplicateSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let src = SessionRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.as_str().to_owned()))?;

        let copy = Session {
            id: SessionId::mint(),
            project_id: src.project_id.clone(),
            title: format!("{} (copy)", src.title),
            title_source: TitleSource::Custom,
            created_at: now_iso(),
            spec_version: src.spec_version,
            spec_json: src.spec_json.clone(),
            sort_order: src.sort_order + 1,
            pinned: false,
            status: SessionStatus::Active,
        };
        SessionRepo::create(cx.db(), &copy).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::apply_launch_spec::ApplyLaunchSpec;
    use crate::app::session::get_session_by_id::GetSessionById;
    use crate::app::session::list_sessions_by_project::ListSessionsByProject;
    use crate::app::session::rename_session::RenameSession;
    use crate::app::session::test_util::{create_one, ctx, unfiled};
    use crate::shared::pagination::Page;

    // Scenario: Duplicating clones the subtree independently
    #[tokio::test]
    async fn duplicate_session_creates_independent_copy() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;

        bus.execute(ApplyLaunchSpec {
            id: id.clone(),
            spec_version: 1,
            spec_json: r#"{"version":1,"items":[]}"#.to_owned(),
        })
        .await
        .unwrap();

        bus.execute(DuplicateSession { id: id.clone() })
            .await
            .unwrap();

        let listing = bus
            .query(ListSessionsByProject {
                project_id: unfiled(),
                page: Page::All,
            })
            .await
            .unwrap();
        assert_eq!(listing.items.len(), 2);

        let copy = listing.items.iter().find(|s| s.id != id).unwrap();
        assert!(copy.title.contains("copy"));
        assert_eq!(copy.spec_version, Some(1));

        bus.execute(RenameSession {
            id: id.clone(),
            title: "Renamed original".to_owned(),
        })
        .await
        .unwrap();
        let copy_after = bus
            .query(GetSessionById {
                id: copy.id.clone(),
            })
            .await
            .unwrap()
            .unwrap();
        assert_ne!(copy_after.title, "Renamed original");
    }
}
