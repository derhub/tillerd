use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::SurfaceId;
use crate::shared::errors::Result;
use crate::shared::message::Command;

// -- DetachSurface (drop the proxy, PTY keeps running) ---------------------------

/// Drop the proxy stream; the PTY keeps running in the daemon. A deliberate,
/// infrequent op -- fine to dispatch and log, so it is a regular bus command.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachSurface {
    pub id: String,
}

impl Command<Ctx> for DetachSurface {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        cx.runtime().detach(&SurfaceId::from_string(&self.id)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::test_util::{harness, one_surface, seed_session, spawn};
    use crate::infra::daemon_pty_api::RuntimeCall;

    // Detach drops the proxy but the PTY keeps running
    #[tokio::test]
    async fn detach_drops_the_proxy_and_leaves_the_pty_running() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-detach").await;
        h.bus.execute(spawn(&session)).await.unwrap();
        let surface = one_surface(&h, &session).await;
        let id = SurfaceId::from_string(&surface.id);

        h.bus
            .execute(DetachSurface {
                id: surface.id.clone(),
            })
            .await
            .unwrap();

        assert!(h.runtime.is_running(&id), "PTY keeps running");
        assert!(h.runtime.calls().contains(&RuntimeCall::Detach(id)));
    }
}
