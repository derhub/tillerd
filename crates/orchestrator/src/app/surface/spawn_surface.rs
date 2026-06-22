use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::entities::{SurfaceKind, SurfaceStatus};
use crate::infra::daemon_pty_api::{Geometry, SpawnRequest};
use crate::infra::SurfaceRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;

use super::common::{default_cwd, DEFAULT_GEOMETRY};

// -- SpawnSurface (D9: persist intent -> effect -> record) -------------------------

/// Add a surface to a session: persist a `pending` row (committed), spawn its PTY
/// lock-free via the runtime port, then record the outcome. The sqlite write lock
/// is never held across the spawn.
///
/// `kind` is the wire string (`terminal`/`diff`); `cols`/`rows` carry the spawn
/// geometry (both present -> explicit geometry, else the default).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnSurface {
    pub session: String,
    pub kind: String,
    pub cwd: Option<String>,
    pub placement: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
}

impl Command<Ctx> for SpawnSurface {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let kind = parse_kind(&self.kind)?;

        // Capability check before any persist: unsupported kinds leave no trace.
        require_terminal(kind)?;

        let geometry = match (self.cols, self.rows) {
            (Some(cols), Some(rows)) => Geometry { cols, rows },
            _ => DEFAULT_GEOMETRY,
        };

        // 1) persist intent -- a single committed write (pool, no transaction)
        let session_id = SessionId::from_string(&self.session);
        let surface = SurfaceRepo::create(
            cx.db(),
            None,
            &session_id,
            kind,
            self.cwd.as_deref(),
            self.placement.as_deref(),
            SurfaceStatus::Pending,
        )
        .await?;

        // 2) run the effect lock-free -- no transaction held
        let request = SpawnRequest {
            surface: surface.id.clone(),
            command: None,
            token: uuid::Uuid::new_v4().to_string(),
            geometry,
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

fn parse_kind(kind: &str) -> Result<SurfaceKind> {
    match kind {
        "terminal" => Ok(SurfaceKind::Terminal),
        "diff" => Ok(SurfaceKind::Diff),
        other => Err(Error::Validation {
            field: "kind",
            reason: format!("unknown surface kind: {other}"),
        }),
    }
}

/// Reject any kind other than Terminal before any side effect is produced.
/// The only supported kind for PTY spawning is Terminal; this is the app-layer
/// capability gate (the daemon is kind-agnostic and must not see unsupported kinds).
fn require_terminal(kind: SurfaceKind) -> Result<()> {
    if kind == SurfaceKind::Terminal {
        Ok(())
    } else {
        Err(Error::Validation {
            field: "kind",
            reason: format!(
                "surface kind '{}' cannot be spawned as a PTY",
                kind.as_str()
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::app::surface::test_util::{harness, one_surface, seed_session, spawn};
    use crate::entities::SurfaceId;
    use crate::infra::daemon_pty_api::RuntimeCall;
    use crate::infra::SurfaceRepo;
    use crate::shared::pagination::Page;

    use super::SpawnSurface;

    // Scenario: A spawn never holds the write lock across the effect; outcome live
    #[tokio::test]
    async fn spawn_persists_intent_then_records_live_after_the_effect() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-spawn").await;

        h.bus.execute(spawn(&session)).await.unwrap();

        let surface = one_surface(&h, &session).await;
        assert_eq!(surface.status, "live");
        let id = SurfaceId::from_string(&surface.id);
        assert!(h.runtime.is_running(&id));
        assert_eq!(h.runtime.calls(), vec![RuntimeCall::Spawn(id)]);
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
        assert_eq!(surface.status, "failed");
        assert!(!h.runtime.is_running(&SurfaceId::from_string(&surface.id)));
    }

    // Scenario: Unsupported kind is rejected before persist -- no row, no runtime call
    #[tokio::test]
    async fn spawn_rejects_unsupported_kind_before_any_effect() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-kind").await;

        let cmd = SpawnSurface {
            session: session.clone(),
            kind: "diff".to_owned(),
            cwd: None,
            placement: None,
            cols: None,
            rows: None,
        };
        let result = h.bus.execute(cmd).await;
        assert!(
            result.is_err(),
            "diff kind must be rejected with a validation error"
        );

        // No row persisted, no runtime call made.
        let rows = SurfaceRepo::list(
            &h.pool,
            &crate::entities::session::SessionId::from_string(&session),
            Page::All,
        )
        .await
        .unwrap();
        assert!(
            rows.items.is_empty(),
            "no row must be persisted for an unsupported kind"
        );
        assert!(
            h.runtime.calls().is_empty(),
            "runtime must not be called for unsupported kind"
        );
    }
}
