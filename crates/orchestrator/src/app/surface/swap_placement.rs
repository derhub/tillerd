use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::entities::Surface;
use crate::infra::SurfaceRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;

/// Atomically swap the placement bindings of two surfaces in a session. Layout
/// tree slots stay put; the surfaces trade slots.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapPlacement {
    pub session: String,
    pub placement_a: String,
    pub placement_b: String,
}

impl Command<Ctx> for SwapPlacement {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let session_id = SessionId::from_string(&self.session);

        // Resolve both sides before opening a transaction: an unknown placement
        // fails here, before any write, so the command leaves no partial change.
        let surface_a = require_placement(cx, &session_id, &self.placement_a).await?;
        let surface_b = require_placement(cx, &session_id, &self.placement_b).await?;

        // `surface_placement` is a UNIQUE(session_id, placement) index (partial: rows
        // with placement IS NULL are excluded), checked per-statement, not deferred.
        // A direct two-way swap collides with itself, so free `a`'s slot through NULL
        // first -- outside the partial index -- before handing it to `b`.
        cx.transaction(async |tx| {
            SurfaceRepo::update_placement(&mut **tx, &surface_a.id, None).await?;
            SurfaceRepo::update_placement(&mut **tx, &surface_b.id, Some(&self.placement_a))
                .await?;
            SurfaceRepo::update_placement(&mut **tx, &surface_a.id, Some(&self.placement_b)).await
        })
        .await
    }
}

async fn require_placement(cx: &Ctx, session_id: &SessionId, placement: &str) -> Result<Surface> {
    SurfaceRepo::find_by_placement(cx.db(), session_id, placement)
        .await?
        .ok_or_else(|| {
            Error::SurfaceNotFound(format!(
                "placement '{placement}' in session '{}'",
                session_id.as_str()
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::test_util::{harness, seed_session};
    use crate::app::surface::SpawnSurface;
    use crate::infra::daemon_pty_api::RuntimeCall;
    use crate::shared::Error;

    async fn spawn_at(session: &str, placement: &str) -> SpawnSurface {
        SpawnSurface {
            session: session.to_owned(),
            kind: "terminal".to_owned(),
            cwd: Some("/work".to_owned()),
            placement: Some(placement.to_owned()),
            cols: None,
            rows: None,
            command_library_ref: None,
            command_executable: None,
            command_args: Vec::new(),
        }
    }

    // Scenario: Swap succeeds atomically -- each surface carries the other's
    // placement and both PTYs keep running.
    #[tokio::test]
    async fn swap_succeeds_atomically_and_keeps_both_ptys_running() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-swap").await;
        h.bus
            .execute(spawn_at(&session, "left").await)
            .await
            .unwrap();
        h.bus
            .execute(spawn_at(&session, "right").await)
            .await
            .unwrap();

        let left =
            SurfaceRepo::find_by_placement(&h.pool, &SessionId::from_string(&session), "left")
                .await
                .unwrap()
                .expect("left surface");
        let right =
            SurfaceRepo::find_by_placement(&h.pool, &SessionId::from_string(&session), "right")
                .await
                .unwrap()
                .expect("right surface");

        h.bus
            .execute(SwapPlacement {
                session: session.clone(),
                placement_a: "left".to_owned(),
                placement_b: "right".to_owned(),
            })
            .await
            .unwrap();

        let after_left = SurfaceRepo::get(&h.pool, &left.id).await.unwrap().unwrap();
        let after_right = SurfaceRepo::get(&h.pool, &right.id).await.unwrap().unwrap();
        assert_eq!(after_left.placement.as_deref(), Some("right"));
        assert_eq!(after_right.placement.as_deref(), Some("left"));

        assert!(h.runtime.is_running(&left.id));
        assert!(h.runtime.is_running(&right.id));
        assert_eq!(
            h.runtime.calls(),
            vec![
                RuntimeCall::Spawn(left.id.clone()),
                RuntimeCall::Spawn(right.id.clone()),
            ]
        );
    }

    // Scenario: Unknown placement fails without change -- neither surface's
    // placement changes.
    #[tokio::test]
    async fn unknown_placement_fails_without_changing_either_surface() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-swap-miss").await;
        h.bus
            .execute(spawn_at(&session, "left").await)
            .await
            .unwrap();

        let left =
            SurfaceRepo::find_by_placement(&h.pool, &SessionId::from_string(&session), "left")
                .await
                .unwrap()
                .expect("left surface");

        let result = h
            .bus
            .execute(SwapPlacement {
                session: session.clone(),
                placement_a: "left".to_owned(),
                placement_b: "no-such-placement".to_owned(),
            })
            .await;
        assert!(matches!(result, Err(Error::SurfaceNotFound(_))));

        let after_left = SurfaceRepo::get(&h.pool, &left.id).await.unwrap().unwrap();
        assert_eq!(
            after_left.placement.as_deref(),
            Some("left"),
            "the resolvable side must not change when the other side is unknown"
        );
    }
}
