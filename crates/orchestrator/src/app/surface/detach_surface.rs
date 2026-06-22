use crate::context::Ctx;
use crate::entities::SurfaceId;
use crate::shared::cqs::Command;
use crate::shared::errors::Result;

// ── DetachSurface (drop the proxy, PTY keeps running) ───────────────────────────

/// Drop the proxy stream; the PTY keeps running in the daemon. A deliberate,
/// infrequent op — fine to dispatch and log, so it is a regular bus command.
#[derive(Debug, Clone)]
pub struct DetachSurface {
    pub id: SurfaceId,
}

impl Command<Ctx> for DetachSurface {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        cx.runtime().detach(&self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::test_util::{harness, one_surface, seed_session, spawn};
    use crate::infra::runtime::RuntimeCall;

    // Detach drops the proxy but the PTY keeps running
    #[tokio::test]
    async fn detach_drops_the_proxy_and_leaves_the_pty_running() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-detach").await;
        h.bus.execute(spawn(&session)).await.unwrap();
        let surface = one_surface(&h, &session).await;

        h.bus
            .execute(DetachSurface {
                id: surface.id.clone(),
            })
            .await
            .unwrap();

        assert!(h.runtime.is_running(&surface.id), "PTY keeps running");
        assert!(h.runtime.calls().contains(&RuntimeCall::Detach(surface.id)));
    }
}
