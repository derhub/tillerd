use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::infra::session::SessionRepo;
use crate::shared::pagination::Page;
use crate::shared::{Command, Result};

/// Stop every running surface across the project's sessions so the project becomes idle.
/// Does not hold a DB transaction across the runtime effect (D9).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopProjectSurfaces {
    pub id: String,
}

impl Command<Ctx> for StopProjectSurfaces {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        use crate::entities::surface::SurfaceStatus;
        use crate::infra::SurfaceRepo;

        let id = ProjectId::new(&self.id);
        let sessions = SessionRepo::list(cx.db(), &id, Page::All).await?;
        for session in sessions.items {
            let surfaces = SurfaceRepo::list(cx.db(), &session.id, Page::All).await?;
            for surface in surfaces.items {
                if surface.status == SurfaceStatus::Live {
                    // Stop the PTY (no DB lock held).
                    cx.runtime().stop(&surface.id).await?;
                    // Record outcome (emits the surface-status push).
                    crate::app::surface::update_status_and_emit(
                        cx,
                        &surface.id,
                        SurfaceStatus::Idle,
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }
}
