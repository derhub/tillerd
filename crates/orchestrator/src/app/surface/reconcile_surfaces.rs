use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::SurfaceStatus;
use crate::infra::daemon_pty_api::SpawnRequest;
use crate::shared::errors::Result;
use crate::shared::message::Command;

use super::common::{all_surfaces, default_cwd, DEFAULT_GEOMETRY};

/// Converge the runtime to the persisted desired state on boot:
/// running-but-no-row -> kill; desired-but-not-running -> respawn or mark failed.
/// It attaches no proxy stream -- streaming is brought up lazily by `attach_surface`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileSurfaces;

impl Command<Ctx> for ReconcileSurfaces {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let live = cx.runtime().list().await?;
        let desired = all_surfaces(cx).await?;

        // running-but-no-row -> kill the orphan PTY
        for id in &live {
            if !desired.iter().any(|s| &s.id == id) {
                cx.runtime().close(id).await?;
            }
        }

        // desired-but-not-running -> respawn, or mark failed if the spawn fails
        for surface in &desired {
            if live.contains(&surface.id) {
                continue;
            }
            let request = SpawnRequest {
                surface: surface.id.clone(),
                command: None,
                token: uuid::Uuid::new_v4().to_string(),
                geometry: DEFAULT_GEOMETRY,
                cwd: surface.cwd.clone().unwrap_or_else(default_cwd),
            };
            match cx.runtime().spawn(request).await {
                Ok(()) => {
                    super::status_events::update_status_and_emit(
                        cx,
                        &surface.id,
                        SurfaceStatus::Live,
                    )
                    .await?
                }
                Err(_) => {
                    super::status_events::update_status_and_emit(
                        cx,
                        &surface.id,
                        SurfaceStatus::Failed,
                    )
                    .await?
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::get_surface_by_id::GetSurfaceById;
    use crate::app::surface::test_util::{harness, seed_session};
    use crate::entities::session::SessionId;
    use crate::entities::{SurfaceId, SurfaceKind};
    use crate::infra::daemon_pty_api::RuntimeCall;
    use crate::infra::SurfaceRepo;

    // Scenario: Boot reconcile kills an orphan PTY (running-but-no-row)
    #[tokio::test]
    async fn reconcile_kills_a_running_pty_with_no_row() {
        let h = harness().await;
        let orphan = SurfaceId::from_string("orphan");
        h.runtime.seed_running(orphan.clone());

        h.bus.execute(ReconcileSurfaces).await.unwrap();

        assert!(!h.runtime.is_running(&orphan), "orphan must be killed");
        assert!(h.runtime.calls().contains(&RuntimeCall::Close(orphan)));
    }

    // Scenario: Boot reconcile respawns a desired row with no live PTY, without attaching
    #[tokio::test]
    async fn reconcile_respawns_a_desired_row_with_no_live_pty_and_attaches_nothing() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-rec").await;
        // a desired row exists but nothing is running in the daemon
        SurfaceRepo::create(
            &h.pool,
            Some("desired"),
            &SessionId::from_string(&session),
            SurfaceKind::Terminal,
            None,
            None,
            crate::entities::SurfaceStatus::Pending,
        )
        .await
        .unwrap();

        h.bus.execute(ReconcileSurfaces).await.unwrap();

        let id = SurfaceId::from_string("desired");
        assert!(h.runtime.is_running(&id), "desired row must be respawned");
        let surface = h
            .bus
            .query(GetSurfaceById {
                id: "desired".to_owned(),
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(surface.status, "live");
        assert!(
            !h.runtime
                .calls()
                .iter()
                .any(|c| matches!(c, RuntimeCall::Attach(_))),
            "reconcile attaches no stream"
        );
    }
}
