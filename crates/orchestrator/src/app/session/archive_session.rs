use crate::context::Ctx;
use crate::entities::session::{SessionId, SessionStatus};
use crate::infra::session::SessionRepo;
use crate::infra::surface_repo::SurfaceRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::{Error, Result};
use crate::shared::pagination::Page;

use super::common::now_iso;

/// Archive a session. Rejects unless the session is idle (no `live` surfaces).
pub struct ArchiveSession {
    pub id: SessionId,
}

impl Command<Ctx> for ArchiveSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let s = SessionRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.as_str().to_owned()))?;

        if s.status == SessionStatus::Archived {
            return Err(Error::SessionAlreadyArchived);
        }

        let surfaces = SurfaceRepo::list(cx.db(), &self.id, Page::All).await?;
        let live_count = surfaces
            .items
            .iter()
            .filter(|sf| sf.status.is_live())
            .count();
        if live_count > 0 {
            return Err(Error::SessionNotIdle(format!(
                "session {} has {} live surface(s)",
                self.id.as_str(),
                live_count,
            )));
        }

        SessionRepo::set_archived(cx.db(), &self.id, &now_iso()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::get_session_by_id::GetSessionById;
    use crate::app::session::test_util::{create_one, ctx};

    // Scenario: Archive is rejected unless all in-scope sessions are idle
    #[tokio::test]
    async fn archive_session_rejects_when_live_surface_exists() {
        let (bus, pool) = ctx().await;
        let id = create_one(&bus).await;

        sqlx::query(
            "INSERT INTO surface (id, session_id, kind, status) VALUES (?, ?, 'terminal', 'live')",
        )
        .bind("surf-live")
        .bind(id.as_str())
        .execute(&pool)
        .await
        .unwrap();

        let err = bus.execute(ArchiveSession { id }).await.unwrap_err();
        assert_eq!(err.code(), "session.not_idle");
    }

    #[tokio::test]
    async fn archive_session_succeeds_when_idle() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;

        bus.execute(ArchiveSession { id: id.clone() })
            .await
            .unwrap();

        let s = bus.query(GetSessionById { id }).await.unwrap().unwrap();
        assert_eq!(s.status, SessionStatus::Archived);
    }

    #[tokio::test]
    async fn archive_session_rejects_already_archived() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;
        bus.execute(ArchiveSession { id: id.clone() })
            .await
            .unwrap();

        let err = bus.execute(ArchiveSession { id }).await.unwrap_err();
        assert_eq!(err.code(), "session.already_archived");
    }
}
