use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::entities::{NewSurface, SurfaceKind, SurfaceStatus};
use crate::infra::runtime::{Geometry, SpawnCommand, SpawnRequest};
use crate::infra::SurfaceRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::Result;

use super::common::{default_cwd, DEFAULT_GEOMETRY};

// ── SpawnSurface (D9: persist intent → effect → record) ─────────────────────────

/// Add a surface to a session: persist a `pending` row (committed), spawn its PTY
/// lock-free via the runtime port, then record the outcome. The sqlite write lock
/// is never held across the spawn.
#[derive(Debug, Clone)]
pub struct SpawnSurface {
    pub session: SessionId,
    pub kind: SurfaceKind,
    pub cwd: Option<String>,
    pub placement: Option<String>,
    pub command: Option<SpawnCommand>,
    pub geometry: Option<Geometry>,
}

impl Command<Ctx> for SpawnSurface {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        // 1) persist intent — a single committed write (pool, no transaction)
        let surface = SurfaceRepo::create(
            cx.db(),
            &NewSurface {
                id: None,
                session_id: self.session.clone(),
                kind: self.kind,
                cwd: self.cwd.clone(),
                placement: self.placement.clone(),
            },
        )
        .await?;

        // 2) run the effect lock-free — no transaction held
        let request = SpawnRequest {
            surface: surface.id.clone(),
            kind: self.kind,
            command: self.command.clone(),
            token: uuid::Uuid::new_v4().to_string(),
            geometry: self.geometry.unwrap_or(DEFAULT_GEOMETRY),
            cwd: self.cwd.clone().unwrap_or_else(default_cwd),
        };
        match cx.runtime().spawn(request).await {
            // 3) record the outcome
            Ok(()) => SurfaceRepo::update_status(cx.db(), &surface.id, SurfaceStatus::Live).await,
            Err(e) => {
                SurfaceRepo::update_status(cx.db(), &surface.id, SurfaceStatus::Failed).await?;
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::test_util::{harness, one_surface, seed_session, spawn};
    use crate::infra::runtime::RuntimeCall;

    // Scenario: A spawn never holds the write lock across the effect; outcome live
    #[tokio::test]
    async fn spawn_persists_intent_then_records_live_after_the_effect() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-spawn").await;

        h.bus.execute(spawn(&session)).await.unwrap();

        let surface = one_surface(&h, &session).await;
        assert_eq!(surface.status, SurfaceStatus::Live);
        assert!(h.runtime.is_running(&surface.id));
        assert_eq!(h.runtime.calls(), vec![RuntimeCall::Spawn(surface.id)]);
    }

    // Scenario: a failed effect records `failed` and is reconcilable, not stranded
    #[tokio::test]
    async fn spawn_records_failed_when_the_effect_fails() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-fail").await;
        h.runtime.fail_next_spawn();

        let result = h.bus.execute(spawn(&session)).await;
        assert!(result.is_err());

        let surface = one_surface(&h, &session).await;
        assert_eq!(surface.status, SurfaceStatus::Failed);
        assert!(!h.runtime.is_running(&surface.id));
    }
}
