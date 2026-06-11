//! Per-surface PTY proxy.
//!
//! [`SurfaceRuntime`] owns one proxy per terminal surface. A proxy opens (or
//! reattaches to) a daemon session keyed by the `surface_id`, fans raw PTY bytes
//! and status to the host [`SurfaceEventSink`], queues input until the session is
//! live, returns flow-control credit, and forwards resize. Detach leaves the
//! daemon session running; removal terminates it.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use contracts::SessionId as WireSessionId;
use daemon_pty_client::{SessionFrame, SpawnParams};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::error::{OrchestratorError, Result};
use crate::persistence::{Store, SurfaceId, SurfaceKind};
use crate::surface::transport::DaemonConnection;

/// Sink the host implements to deliver a surface's output to a client. Every
/// method is called from the runtime's async task and MUST NOT block.
pub trait SurfaceEventSink: Send + Sync + 'static {
    /// Raw PTY output bytes for a surface, delivered unchanged.
    fn on_bytes(&self, surface: &SurfaceId, bytes: &[u8]);
    /// A terminal-status transition for a surface.
    fn on_status(&self, surface: &SurfaceId, status: &str);
    /// The surface's PTY exited with the given qualifier.
    fn on_exit(&self, surface: &SurfaceId, qualifier: &str);
}

/// The daemon socket connection state of one surface proxy.
enum ProxyState {
    /// The session is being spawned; input is buffered until `spawn-ack`.
    Attaching(Vec<Vec<u8>>),
    /// The session is live; input is forwarded immediately.
    Live,
    /// The session has exited or been removed.
    Closed,
}

/// A live proxy: the daemon connection, its attach state, and the read task.
struct Proxy {
    conn: Arc<DaemonConnection>,
    state: Arc<Mutex<ProxyState>>,
    task: JoinHandle<()>,
}

/// Shared context the read task and frame handler operate on.
struct ProxyCtx {
    surface: SurfaceId,
    wire: WireSessionId,
    conn: Arc<DaemonConnection>,
    sink: Arc<dyn SurfaceEventSink>,
    store: Arc<dyn Store>,
    state: Arc<Mutex<ProxyState>>,
}

/// Owns one PTY proxy per terminal surface over the daemon socket.
pub struct SurfaceRuntime {
    store: Arc<dyn Store>,
    sink: Arc<dyn SurfaceEventSink>,
    socket: PathBuf,
    proxies: Mutex<HashMap<SurfaceId, Proxy>>,
}

impl SurfaceRuntime {
    /// Build a runtime that connects to the daemon at `socket`.
    pub fn new(store: Arc<dyn Store>, sink: Arc<dyn SurfaceEventSink>, socket: PathBuf) -> Self {
        Self {
            store,
            sink,
            socket,
            proxies: Mutex::new(HashMap::new()),
        }
    }

    /// The number of live proxies (one per attached surface).
    pub async fn proxy_count(&self) -> usize {
        self.proxies.lock().await.len()
    }

    /// Open a fresh terminal surface: spawn a daemon session keyed by `surface`
    /// and start streaming. Input sent before the spawn is acknowledged is
    /// queued and flushed in order.
    pub async fn open_terminal(
        &self,
        surface: SurfaceId,
        token: String,
        cols: u16,
        rows: u16,
        cwd: String,
    ) -> Result<()> {
        if self.proxies.lock().await.contains_key(&surface) {
            return Err(surface_err(&surface, "surface already has a proxy"));
        }

        let wire = wire_id(&surface);
        let (conn, rx) = DaemonConnection::connect(&self.socket)
            .await
            .map_err(|e| surface_err(&surface, e))?;
        let conn = Arc::new(conn);

        let params = SpawnParams {
            session_id: &wire,
            token: &token,
            cols,
            rows,
            cwd: &cwd,
            command: None,
            args: &[],
            env: None,
            resume: None,
        };
        conn.spawn(&params)
            .await
            .map_err(|e| surface_err(&surface, e))?;

        let state = Arc::new(Mutex::new(ProxyState::Attaching(Vec::new())));
        self.spawn_proxy(surface, wire, conn, rx, state).await;
        Ok(())
    }

    /// Reattach to a surface whose daemon session is still alive, by `surface`.
    /// Returns a typed error if the daemon has no such session.
    pub async fn resume(&self, surface: SurfaceId) -> Result<()> {
        if self.proxies.lock().await.contains_key(&surface) {
            return Ok(());
        }

        let wire = wire_id(&surface);
        let (conn, mut rx) = DaemonConnection::connect(&self.socket)
            .await
            .map_err(|e| surface_err(&surface, e))?;
        let conn = Arc::new(conn);
        conn.subscribe(&wire)
            .await
            .map_err(|e| surface_err(&surface, e))?;

        // The daemon answers a subscribe with a snapshot/replay + status for a
        // live session, or an error frame for a missing one.
        let first = rx
            .recv()
            .await
            .ok_or_else(|| surface_err(&surface, "daemon closed during resume"))?;

        let state = Arc::new(Mutex::new(ProxyState::Live));
        let ctx = self.ctx(surface.clone(), wire, conn.clone(), state.clone());
        if let SessionFrame::Error { code, message, .. } = &first {
            return Err(surface_err(
                &surface,
                format!("session not resumable: {code} {message}"),
            ));
        }
        if !handle_frame(&ctx, first).await {
            return Err(surface_err(&surface, "session exited during resume"));
        }

        let task = tokio::spawn(read_loop(ctx, rx));
        self.proxies.lock().await.insert(
            surface,
            Proxy {
                conn,
                state,
                task,
            },
        );
        Ok(())
    }

    /// Resume every persisted terminal surface whose daemon session survives.
    /// Surfaces whose session is gone are skipped (not fatal to boot).
    pub async fn resume_all(&self) -> Result<()> {
        let surfaces = self.store.list_resumable_surfaces()?;
        for surface in surfaces {
            if surface.kind != SurfaceKind::Terminal {
                continue;
            }
            let _ = self.resume(surface.id).await;
        }
        Ok(())
    }

    /// Forward input bytes to a surface's PTY, queueing while it is attaching.
    pub async fn input(&self, surface: &SurfaceId, bytes: &[u8]) -> Result<()> {
        let (conn, state) = self.handle(surface).await?;
        let mut st = state.lock().await;
        match &mut *st {
            ProxyState::Attaching(queue) => {
                queue.push(bytes.to_vec());
                Ok(())
            }
            ProxyState::Live => {
                drop(st);
                conn.input(&wire_id(surface), bytes)
                    .await
                    .map_err(|e| surface_err(surface, e))
            }
            ProxyState::Closed => Err(surface_err(surface, "surface is closed")),
        }
    }

    /// Resize a surface's terminal.
    pub async fn resize(&self, surface: &SurfaceId, cols: u16, rows: u16) -> Result<()> {
        let (conn, _) = self.handle(surface).await?;
        conn.resize(&wire_id(surface), cols, rows)
            .await
            .map_err(|e| surface_err(surface, e))
    }

    /// Detach from a surface: stop streaming but leave the daemon session alive
    /// so the surface can resume later.
    pub async fn detach(&self, surface: &SurfaceId) -> Result<()> {
        let Some(proxy) = self.proxies.lock().await.remove(surface) else {
            return Ok(());
        };
        let _ = proxy.conn.unsubscribe(&wire_id(surface)).await;
        proxy.task.abort();
        Ok(())
    }

    /// Remove a surface: terminate its daemon session and release the proxy.
    pub async fn remove(&self, surface: &SurfaceId) -> Result<()> {
        let Some(proxy) = self.proxies.lock().await.remove(surface) else {
            return Ok(());
        };
        let _ = proxy.conn.kill(&wire_id(surface)).await;
        proxy.task.abort();
        Ok(())
    }

    // ── internal helpers ────────────────────────────────────────────────────

    /// Clone the connection + state handles for a surface, releasing the map
    /// lock before any await on them.
    async fn handle(
        &self,
        surface: &SurfaceId,
    ) -> Result<(Arc<DaemonConnection>, Arc<Mutex<ProxyState>>)> {
        let proxies = self.proxies.lock().await;
        let proxy = proxies
            .get(surface)
            .ok_or_else(|| surface_err(surface, "no such surface"))?;
        Ok((proxy.conn.clone(), proxy.state.clone()))
    }

    fn ctx(
        &self,
        surface: SurfaceId,
        wire: WireSessionId,
        conn: Arc<DaemonConnection>,
        state: Arc<Mutex<ProxyState>>,
    ) -> ProxyCtx {
        ProxyCtx {
            surface,
            wire,
            conn,
            sink: self.sink.clone(),
            store: self.store.clone(),
            state,
        }
    }

    async fn spawn_proxy(
        &self,
        surface: SurfaceId,
        wire: WireSessionId,
        conn: Arc<DaemonConnection>,
        rx: tokio::sync::mpsc::Receiver<SessionFrame>,
        state: Arc<Mutex<ProxyState>>,
    ) {
        let ctx = self.ctx(surface.clone(), wire, conn.clone(), state.clone());
        let task = tokio::spawn(read_loop(ctx, rx));
        self.proxies.lock().await.insert(
            surface,
            Proxy {
                conn,
                state,
                task,
            },
        );
    }
}

/// The daemon session id for a surface: the `surface_id` verbatim (ADR-0020).
fn wire_id(surface: &SurfaceId) -> WireSessionId {
    WireSessionId(surface.as_str().to_string())
}

fn surface_err(surface: &SurfaceId, reason: impl std::fmt::Display) -> OrchestratorError {
    OrchestratorError::Surface {
        surface: surface.as_str().to_string(),
        reason: reason.to_string(),
    }
}

/// Consume daemon frames until the stream ends or the session exits.
async fn read_loop(ctx: ProxyCtx, mut rx: tokio::sync::mpsc::Receiver<SessionFrame>) {
    while let Some(frame) = rx.recv().await {
        if !handle_frame(&ctx, frame).await {
            break;
        }
    }
}

/// Handle one decoded frame. Returns `false` once the session has exited.
async fn handle_frame(ctx: &ProxyCtx, frame: SessionFrame) -> bool {
    match frame {
        SessionFrame::SpawnAck { .. } => {
            let queued = {
                let mut st = ctx.state.lock().await;
                match std::mem::replace(&mut *st, ProxyState::Live) {
                    ProxyState::Attaching(queue) => queue,
                    other => {
                        *st = other_to_live(other);
                        Vec::new()
                    }
                }
            };
            for chunk in queued {
                let _ = ctx.conn.input(&ctx.wire, &chunk).await;
            }
            true
        }
        SessionFrame::Data { bytes, .. } => {
            ctx.sink.on_bytes(&ctx.surface, &bytes);
            let _ = ctx.conn.ack(&ctx.wire, bytes.len() as i64).await;
            true
        }
        SessionFrame::Status { status, .. } => {
            ctx.sink.on_status(&ctx.surface, &status);
            let _ = ctx.store.update_surface_status(&ctx.surface, &status);
            true
        }
        SessionFrame::Exit { qualifier, .. } => {
            ctx.sink.on_exit(&ctx.surface, &qualifier);
            *ctx.state.lock().await = ProxyState::Closed;
            false
        }
        SessionFrame::Error { .. }
        | SessionFrame::HelloAck { .. }
        | SessionFrame::Other { .. } => true,
    }
}

/// Preserve a non-`Attaching` state when a stray `spawn-ack` arrives.
fn other_to_live(state: ProxyState) -> ProxyState {
    match state {
        ProxyState::Closed => ProxyState::Closed,
        _ => ProxyState::Live,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::memory::InMemoryStore;
    use daemon_pty_client::{encode_frame, FrameDecoder};
    use std::sync::Mutex as StdMutex;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{UnixListener, UnixStream};

    #[derive(Default)]
    struct CollectingSink {
        bytes: StdMutex<Vec<u8>>,
        statuses: StdMutex<Vec<String>>,
        exits: StdMutex<Vec<String>>,
    }

    impl SurfaceEventSink for CollectingSink {
        fn on_bytes(&self, _surface: &SurfaceId, bytes: &[u8]) {
            self.bytes.lock().unwrap().extend_from_slice(bytes);
        }
        fn on_status(&self, _surface: &SurfaceId, status: &str) {
            self.statuses.lock().unwrap().push(status.to_string());
        }
        fn on_exit(&self, _surface: &SurfaceId, qualifier: &str) {
            self.exits.lock().unwrap().push(qualifier.to_string());
        }
    }

    /// Read one full frame from `rx` into `decoder`, blocking until available.
    async fn read_frame(rx: &mut tokio::net::unix::OwnedReadHalf, decoder: &mut FrameDecoder) -> RawFrameMeta {
        let mut buf = vec![0u8; 1024];
        loop {
            let n = rx.read(&mut buf).await.expect("read");
            assert!(n > 0, "socket closed");
            let frames = decoder.push(&buf[..n]);
            if let Some(f) = frames.into_iter().next() {
                let meta: serde_json::Value = serde_json::from_slice(&f.meta).expect("meta json");
                return RawFrameMeta { ty: meta["type"].as_str().unwrap_or("").to_string() };
            }
        }
    }

    struct RawFrameMeta {
        ty: String,
    }

    fn hello_ack() -> Vec<u8> {
        encode_frame(
            br#"{"type":"hello-ack","version":1,"daemonVersion":"0.0.0","capabilities":["snapshot"]}"#,
            None,
        )
    }

    async fn accept(listener: UnixListener) -> UnixStream {
        let (stream, _) = listener.accept().await.expect("accept");
        stream
    }

    #[tokio::test]
    async fn open_terminal_streams_bytes_and_status_and_acks() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();

        let daemon = tokio::spawn(async move {
            let stream = accept(listener).await;
            let (mut rx, mut tx) = stream.into_split();
            let mut dec = FrameDecoder::new();
            assert_eq!(read_frame(&mut rx, &mut dec).await.ty, "hello");
            tx.write_all(&hello_ack()).await.unwrap();
            assert_eq!(read_frame(&mut rx, &mut dec).await.ty, "spawn");
            tx.write_all(&encode_frame(br#"{"type":"spawn-ack","sessionId":"surf-1","pid":7}"#, None))
                .await
                .unwrap();
            tx.write_all(&encode_frame(br#"{"type":"data","sessionId":"surf-1"}"#, Some(b"hi")))
                .await
                .unwrap();
            // The proxy must return credit for the 2 bytes it consumed.
            assert_eq!(read_frame(&mut rx, &mut dec).await.ty, "ack");
            tx.write_all(&encode_frame(
                br#"{"type":"status","sessionId":"surf-1","status":"WORKING","source":"terminal"}"#,
                None,
            ))
            .await
            .unwrap();
            // Keep the connection open briefly so the proxy can drain.
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        });

        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let sink = Arc::new(CollectingSink::default());
        let runtime = SurfaceRuntime::new(store, sink.clone(), sock);

        runtime
            .open_terminal(SurfaceId::from_string("surf-1"), "tok".into(), 80, 24, "/tmp".into())
            .await
            .expect("open");

        // Allow the read task to process the scripted frames.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(runtime.proxy_count().await, 1);
        assert_eq!(&*sink.bytes.lock().unwrap(), b"hi");
        assert_eq!(&*sink.statuses.lock().unwrap(), &["WORKING".to_string()]);
        daemon.abort();
    }

    #[tokio::test]
    async fn resume_errors_when_session_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();

        let daemon = tokio::spawn(async move {
            let stream = accept(listener).await;
            let (mut rx, mut tx) = stream.into_split();
            let mut dec = FrameDecoder::new();
            assert_eq!(read_frame(&mut rx, &mut dec).await.ty, "hello");
            tx.write_all(&hello_ack()).await.unwrap();
            assert_eq!(read_frame(&mut rx, &mut dec).await.ty, "subscribe");
            tx.write_all(&encode_frame(
                br#"{"type":"error","code":"ENOENT","message":"unknown session","sessionId":"gone"}"#,
                None,
            ))
            .await
            .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        });

        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let sink = Arc::new(CollectingSink::default());
        let runtime = SurfaceRuntime::new(store, sink, sock);

        let result = runtime.resume(SurfaceId::from_string("gone")).await;
        assert!(matches!(result, Err(OrchestratorError::Surface { .. })));
        assert_eq!(runtime.proxy_count().await, 0);
        daemon.abort();
    }

    #[tokio::test]
    async fn input_is_queued_until_spawn_ack_then_flushed() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();

        let daemon = tokio::spawn(async move {
            let stream = accept(listener).await;
            let (mut rx, mut tx) = stream.into_split();
            let mut dec = FrameDecoder::new();
            assert_eq!(read_frame(&mut rx, &mut dec).await.ty, "hello");
            tx.write_all(&hello_ack()).await.unwrap();
            assert_eq!(read_frame(&mut rx, &mut dec).await.ty, "spawn");
            // Delay the spawn-ack so the test's input() call is queued.
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            tx.write_all(&encode_frame(br#"{"type":"spawn-ack","sessionId":"s","pid":1}"#, None))
                .await
                .unwrap();
            // After the ack the queued input must arrive.
            assert_eq!(read_frame(&mut rx, &mut dec).await.ty, "input");
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        });

        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let sink = Arc::new(CollectingSink::default());
        let runtime = SurfaceRuntime::new(store, sink, sock);

        let surface = SurfaceId::from_string("s");
        runtime
            .open_terminal(surface.clone(), "t".into(), 80, 24, "/".into())
            .await
            .expect("open");
        // Send input while still attaching (spawn-ack delayed).
        runtime.input(&surface, b"ls\n").await.expect("input queued");

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        daemon.abort();
    }
}
