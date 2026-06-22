use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::infra::surface_repo::SurfaceRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;
use crate::shared::pagination::Page;

/// Hard-delete a session. A forceful remove: kills any running surfaces (so no PTY is
/// orphaned), then deletes the session -- `ON DELETE CASCADE` drops its surface rows.
/// Unlike `ArchiveSession`, this does not require the session to be idle.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardSession {
    pub id: String,
}

impl Command<Ctx> for DiscardSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = SessionId::from_string(&self.id);
        SessionRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.clone()))?;

        let surfaces = SurfaceRepo::list(cx.db(), &id, Page::All).await?;
        for sf in surfaces.items.iter().filter(|sf| sf.status.is_live()) {
            cx.runtime().stop(&sf.id).await?;
        }

        SessionRepo::delete(cx.db(), &id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::archive_session::ArchiveSession;
    use crate::app::session::get_session_by_id::GetSessionById;
    use crate::app::session::test_util::{create_one, ctx};

    // Scenario: hard-delete of an active session removes it without an archive precondition
    #[tokio::test]
    async fn discard_active_session_removes_it() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;
        bus.execute(DiscardSession { id: id.clone() })
            .await
            .unwrap();
        let got = bus.query(GetSessionById { id }).await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn discard_session_deletes_archived_session() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;
        bus.execute(ArchiveSession { id: id.clone() })
            .await
            .unwrap();
        bus.execute(DiscardSession { id: id.clone() })
            .await
            .unwrap();
        let got = bus.query(GetSessionById { id }).await.unwrap();
        assert!(got.is_none());
    }
}
