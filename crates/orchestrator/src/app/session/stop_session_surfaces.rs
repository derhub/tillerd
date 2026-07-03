use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::surface_repo::SurfaceRepo;
use crate::shared::errors::Result;
use crate::shared::message::Command;
use crate::shared::pagination::Page;

/// Mark every live surface in the session as `Idle`. DB-only; caller drives runtime separately.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopSessionSurfaces {
    pub id: String,
}

impl Command<Ctx> for StopSessionSurfaces {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = SessionId::from_string(&self.id);
        let surfaces = SurfaceRepo::list(cx.db(), &id, Page::All).await?;
        let live: Vec<_> = surfaces
            .items
            .iter()
            .filter(|sf| sf.status.is_live())
            .collect();
        if live.is_empty() {
            return Ok(());
        }
        let workspace_id = crate::app::surface::workspace_id_for_session(cx, &id).await?;
        for sf in live {
            crate::app::surface::update_status_and_emit(
                cx,
                &sf.id,
                &workspace_id,
                crate::entities::surface::SurfaceStatus::Idle,
            )
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::archive_session::ArchiveSession;
    use crate::app::session::test_util::{create_one, ctx};
    use crate::infra::surface_repo::SurfaceRepo;
    use crate::shared::pagination::Page;

    // Scenario: Stopping a scope makes it idle
    #[tokio::test]
    async fn stop_session_surfaces_marks_live_surfaces_idle() {
        let (bus, pool) = ctx().await;
        let id = create_one(&bus).await;

        sqlx::query(
            "INSERT INTO surface (id, session_id, kind, status) VALUES (?, ?, 'terminal', 'live')",
        )
        .bind("surf-s1")
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();

        bus.execute(StopSessionSurfaces { id: id.clone() })
            .await
            .unwrap();

        let listing = SurfaceRepo::list(&pool, &SessionId::from_string(&id), Page::All)
            .await
            .unwrap();
        assert!(
            !listing.items.iter().any(|sf| sf.status.is_live()),
            "no live surfaces after stop"
        );

        bus.execute(ArchiveSession { id }).await.unwrap();
    }
}
