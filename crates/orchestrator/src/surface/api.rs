use std::path::PathBuf;
use std::sync::Arc;

use crate::error::Result;
use crate::persistence::{NewSurface, SessionId, Store, SurfaceId, SurfaceKind};
use crate::surface::runtime::{SurfaceEventSink, SurfaceRuntime};

pub struct SurfaceApi {
    runtime: Arc<SurfaceRuntime>,
    store: Arc<dyn Store>,
}

impl SurfaceApi {
    pub fn new(store: Arc<dyn Store>, sink: Arc<dyn SurfaceEventSink>, socket: PathBuf) -> Self {
        let runtime = Arc::new(SurfaceRuntime::new(store.clone(), sink, socket));
        Self { runtime, store }
    }

    pub async fn create_terminal_surface(
        &self,
        session_id: SessionId,
        surface_id: SurfaceId,
        cols: u16,
        rows: u16,
        cwd: Option<String>,
    ) -> Result<SurfaceId> {
        let surface = self.store.create_surface(NewSurface {
            id: Some(surface_id.clone()),
            session_id,
            kind: SurfaceKind::Terminal,
            cwd: cwd.clone(),
            placement: None,
            worktree_id: None,
        })?;

        let token = uuid::Uuid::new_v4().to_string();
        let work_dir = cwd.unwrap_or_else(default_cwd);
        self.runtime
            .launch_surface(
                surface.id.clone(),
                SurfaceKind::Terminal,
                None,
                token,
                cols,
                rows,
                work_dir,
            )
            .await?;
        Ok(surface.id)
    }

    pub async fn input(&self, surface: &SurfaceId, bytes: &[u8]) -> Result<()> {
        self.runtime.input(surface, bytes).await
    }

    pub async fn resize(&self, surface: &SurfaceId, cols: u16, rows: u16) -> Result<()> {
        self.runtime.resize(surface, cols, rows).await
    }

    pub async fn resume_all(&self) -> Result<()> {
        self.runtime.resume_all().await
    }

    pub async fn detach(&self, surface: &SurfaceId) -> Result<()> {
        self.runtime.detach(surface).await
    }

    pub async fn remove(&self, surface: &SurfaceId) -> Result<()> {
        self.store.soft_delete_surface(surface)?;
        self.runtime.remove(surface).await
    }
}

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

    async fn fake_daemon(listener: UnixListener) {
        let (stream, _) = listener.accept().await.expect("accept");
        let (mut rx, mut tx) = stream.into_split();
        let mut buf = vec![0u8; 1024];
        let mut dec = FrameDecoder::new();
        loop {
            let Ok(n) = rx.read(&mut buf).await else {
                return;
            };
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
        let session = store
            .create_session(crate::persistence::NewSession::default())
            .unwrap();
        let api = SurfaceApi::new(store.clone(), Arc::new(NullSink), sock);

        let id = SurfaceId::from_string("surf-api-1");
        let returned = api
            .create_terminal_surface(session.id, id.clone(), 80, 24, Some("/tmp".into()))
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
