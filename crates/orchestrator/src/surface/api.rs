//! The orchestrator's terminal-surface API: the operations a host binds to its
//! transport (ADR-0022). It resolves a session, persists the surface row, and
//! drives the [`SurfaceRuntime`] proxy.

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::Result;
use crate::persistence::{NewSession, NewSurface, Store, SurfaceId, SurfaceKind};
use crate::surface::runtime::{SurfaceEventSink, SurfaceRuntime};

/// The terminal-surface API the host calls. Holds the runtime and the store.
pub struct SurfaceApi {
    runtime: Arc<SurfaceRuntime>,
    store: Arc<dyn Store>,
}

impl SurfaceApi {
    /// Build the API over a shared store, a host event sink, and the daemon
    /// socket the runtime connects to.
    pub fn new(store: Arc<dyn Store>, sink: Arc<dyn SurfaceEventSink>, socket: PathBuf) -> Self {
        let runtime = Arc::new(SurfaceRuntime::new(store.clone(), sink, socket));
        Self { runtime, store }
    }

    /// Create a terminal surface under the given (or seeded default) project,
    /// persist its row, and start streaming. The host supplies the `surface_id`
    /// so it can route bytes before the first frame arrives. Returns the id.
    pub async fn create_terminal_surface(
        &self,
        surface_id: SurfaceId,
        cols: u16,
        rows: u16,
        cwd: Option<String>,
    ) -> Result<SurfaceId> {
        // A session without an explicit project belongs to the seeded Unfiled
        // project (workspace-persistence: seeded default project).
        let session = self.store.create_session(NewSession::default())?;
        let surface = self.store.create_surface(NewSurface {
            id: Some(surface_id.clone()),
            session_id: session.id,
            kind: SurfaceKind::Terminal,
            cwd: cwd.clone(),
        })?;

        let token = uuid::Uuid::new_v4().to_string();
        let work_dir = cwd.unwrap_or_else(default_cwd);
        self.runtime
            .open_terminal(surface.id.clone(), token, cols, rows, work_dir)
            .await?;
        Ok(surface.id)
    }

    /// Forward input to a surface's PTY.
    pub async fn input(&self, surface: &SurfaceId, bytes: &[u8]) -> Result<()> {
        self.runtime.input(surface, bytes).await
    }

    /// Resize a surface's terminal.
    pub async fn resize(&self, surface: &SurfaceId, cols: u16, rows: u16) -> Result<()> {
        self.runtime.resize(surface, cols, rows).await
    }

    /// Reattach every persisted terminal surface whose daemon session survives.
    pub async fn resume_all(&self) -> Result<()> {
        self.runtime.resume_all().await
    }

    /// Detach a surface, leaving its daemon session alive for later resume.
    pub async fn detach(&self, surface: &SurfaceId) -> Result<()> {
        self.runtime.detach(surface).await
    }

    /// Remove a surface, terminating its daemon session.
    pub async fn remove(&self, surface: &SurfaceId) -> Result<()> {
        self.store.soft_delete_surface(surface)?;
        self.runtime.remove(surface).await
    }
}

/// The working directory for a terminal whose surface did not specify one.
fn default_cwd() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::memory::InMemoryStore;
    use crate::surface::runtime::SurfaceEventSink;
    use daemon_pty_client::{encode_frame, FrameDecoder};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixListener;

    struct NullSink;
    impl SurfaceEventSink for NullSink {
        fn on_bytes(&self, _surface: &SurfaceId, _bytes: &[u8]) {}
        fn on_status(&self, _surface: &SurfaceId, _status: &str) {}
        fn on_exit(&self, _surface: &SurfaceId, _qualifier: &str) {}
    }

    /// A fake daemon that completes the hello + spawn handshake on one connection.
    async fn fake_daemon(listener: UnixListener) {
        let (stream, _) = listener.accept().await.expect("accept");
        let (mut rx, mut tx) = stream.into_split();
        let mut buf = vec![0u8; 1024];
        let mut dec = FrameDecoder::new();
        // Drain frames; reply hello-ack after hello and spawn-ack after spawn.
        loop {
            let Ok(n) = rx.read(&mut buf).await else { return };
            if n == 0 {
                return;
            }
            for frame in dec.push(&buf[..n]) {
                let meta: serde_json::Value =
                    serde_json::from_slice(&frame.meta).expect("meta json");
                match meta["type"].as_str() {
                    Some("hello") => {
                        let _ = tx
                            .write_all(&encode_frame(
                                br#"{"type":"hello-ack","version":1,"daemonVersion":"0","capabilities":["snapshot"]}"#,
                                None,
                            ))
                            .await;
                    }
                    Some("spawn") => {
                        let id = meta["sessionId"].as_str().unwrap_or("");
                        let ack = format!(r#"{{"type":"spawn-ack","sessionId":"{id}","pid":1}}"#);
                        let _ = tx.write_all(&encode_frame(ack.as_bytes(), None)).await;
                    }
                    _ => {}
                }
            }
        }
    }

    #[tokio::test]
    async fn create_terminal_surface_persists_a_row_and_starts_one_proxy() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let daemon = tokio::spawn(fake_daemon(listener));

        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let api = SurfaceApi::new(store.clone(), Arc::new(NullSink), sock);

        let id = SurfaceId::from_string("surf-api-1");
        let returned = api
            .create_terminal_surface(id.clone(), 80, 24, Some("/tmp".into()))
            .await
            .expect("create");

        assert_eq!(returned, id);
        let row = store.get_surface(&id).expect("get").expect("row exists");
        assert_eq!(row.kind, SurfaceKind::Terminal);
        assert_eq!(row.cwd.as_deref(), Some("/tmp"));
        assert_eq!(api.runtime.proxy_count().await, 1);
        daemon.abort();
    }
}
