use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::surface_repo::SurfaceRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::Result;
use crate::shared::pagination::Page;

/// Mark every live surface in the session as `Idle`. DB-only; caller drives runtime separately.
pub struct StopSessionSurfaces {
    pub id: SessionId,
}

impl Command<Ctx> for StopSessionSurfaces {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let surfaces = SurfaceRepo::list(cx.db(), &self.id, Page::All).await?;
        for sf in surfaces.items.iter().filter(|sf| sf.status.is_live()) {
            SurfaceRepo::update_status(
                cx.db(),
                &sf.id,
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
        .bind(id.as_str())
        .execute(&pool)
        .await
        .unwrap();

        bus.execute(StopSessionSurfaces { id: id.clone() })
            .await
            .unwrap();

        let listing = SurfaceRepo::list(&pool, &id, Page::All).await.unwrap();
        assert!(
            !listing.items.iter().any(|sf| sf.status.is_live()),
            "no live surfaces after stop"
        );

        bus.execute(ArchiveSession { id }).await.unwrap();
    }
}
