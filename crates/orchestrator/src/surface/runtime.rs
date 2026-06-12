use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use contracts::SessionId as WireSessionId;
use daemon_pty_client::{SessionFrame, SpawnParams};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::error::{OrchestratorError, Result};
use crate::persistence::{Store, SurfaceId, SurfaceKind};
use crate::surface::transport::DaemonConnection;

/// A fully resolved launch command: a concrete executable, its arguments, and extra
/// environment. `None` at a spawn site means the login shell.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedCommand {
    pub exe: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

pub trait SurfaceEventSink: Send + Sync + 'static {
    fn on_bytes(&self, surface: &SurfaceId, bytes: &[u8]);
    fn on_status(&self, surface: &SurfaceId, status: &str);
    fn on_exit(&self, surface: &SurfaceId, qualifier: &str);
    /// Called when a non-recoverable surface-level error occurs after open.
    fn on_error(&self, _surface: &SurfaceId, _reason: &str) {}
}

enum ProxyState {
    Attaching(Vec<Vec<u8>>),
    Live,
    Closed,
}

struct TerminalProxy {
    conn: Arc<DaemonConnection>,
    state: Arc<Mutex<ProxyState>>,
    task: JoinHandle<()>,
}

struct ProxyCtx {
    surface: SurfaceId,
    wire: WireSessionId,
    conn: Arc<DaemonConnection>,
    sink: Arc<dyn SurfaceEventSink>,
    store: Arc<dyn Store>,
    state: Arc<Mutex<ProxyState>>,
}

pub struct SurfaceRuntime {
    store: Arc<dyn Store>,
    sink: Arc<dyn SurfaceEventSink>,
    socket: PathBuf,
    proxies: Mutex<HashMap<SurfaceId, TerminalProxy>>,
}

impl SurfaceRuntime {
    pub fn new(store: Arc<dyn Store>, sink: Arc<dyn SurfaceEventSink>, socket: PathBuf) -> Self {
        Self {
            store,
            sink,
            socket,
            proxies: Mutex::new(HashMap::new()),
        }
    }

    pub async fn proxy_count(&self) -> usize {
        self.proxies.lock().await.len()
    }

    /// Generic spawn shared by every surface kind: connect to the pseudo-terminal service,
    /// spawn `command` (or the login shell when `None`), and return the live connection and
    /// frame stream. `surface_id` is the daemon session key (ADR-0024).
    async fn spawn(
        &self,
        surface: &SurfaceId,
        command: Option<&ResolvedCommand>,
        cwd: &str,
        cols: u16,
        rows: u16,
        token: &str,
    ) -> Result<(
        WireSessionId,
        Arc<DaemonConnection>,
        tokio::sync::mpsc::Receiver<SessionFrame>,
    )> {
        let wire = wire_id(surface);
        let (conn, rx) = DaemonConnection::connect(&self.socket)
            .await
            .map_err(|e| surface_err(surface, e))?;
        let conn = Arc::new(conn);

        const NO_ARGS: &[String] = &[];
        let params = SpawnParams {
            session_id: &wire,
            token,
            cols,
            rows,
            cwd,
            command: command.map(|c| c.exe.as_str()),
            args: command.map(|c| c.args.as_slice()).unwrap_or(NO_ARGS),
            env: command.map(|c| &c.env),
            resume: None,
        };
        conn.spawn(&params)
            .await
            .map_err(|e| surface_err(surface, e))?;
        Ok((wire, conn, rx))
    }

    /// Uniform surface launch: dispatch by kind to the kind's adapter. The launch executor
    /// and the surface API call only this; a kind with no adapter is a typed error.
    #[expect(
        clippy::too_many_arguments,
        reason = "the full surface-spawn input set: kind, command, token, terminal size, and cwd"
    )]
    pub async fn launch_surface(
        &self,
        surface: SurfaceId,
        kind: SurfaceKind,
        command: Option<ResolvedCommand>,
        token: String,
        cols: u16,
        rows: u16,
        cwd: String,
    ) -> Result<()> {
        if self.proxies.lock().await.contains_key(&surface) {
            return Err(surface_err(&surface, "surface already has a proxy"));
        }
        match kind {
            SurfaceKind::Terminal => {
                self.launch_terminal(surface, command, token, cols, rows, cwd)
                    .await
            }
            SurfaceKind::Diff => Err(surface_err(
                &surface,
                "unsupported surface kind for launch: diff",
            )),
        }
    }

    async fn launch_terminal(
        &self,
        surface: SurfaceId,
        command: Option<ResolvedCommand>,
        token: String,
        cols: u16,
        rows: u16,
        cwd: String,
    ) -> Result<()> {
        let (wire, conn, rx) = self
            .spawn(&surface, command.as_ref(), &cwd, cols, rows, &token)
            .await?;
        let state = Arc::new(Mutex::new(ProxyState::Attaching(Vec::new())));
        self.spawn_terminal_proxy(surface, wire, conn, rx, state)
            .await;
        Ok(())
    }

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

        // subscribe replies with snapshot+status for a live session, or an error frame for a missing one.
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
        self.proxies
            .lock()
            .await
            .insert(surface, TerminalProxy { conn, state, task });
        Ok(())
    }

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

    pub async fn resize(&self, surface: &SurfaceId, cols: u16, rows: u16) -> Result<()> {
        let (conn, _) = self.handle(surface).await?;
        conn.resize(&wire_id(surface), cols, rows)
            .await
            .map_err(|e| surface_err(surface, e))
    }

    pub async fn detach(&self, surface: &SurfaceId) -> Result<()> {
        let Some(p) = self.proxies.lock().await.remove(surface) else {
            return Ok(());
        };
        let _ = p.conn.unsubscribe(&wire_id(surface)).await;
        p.task.abort();
        Ok(())
    }

    pub async fn remove(&self, surface: &SurfaceId) -> Result<()> {
        let Some(p) = self.proxies.lock().await.remove(surface) else {
            return Ok(());
        };
        let _ = p.conn.kill(&wire_id(surface)).await;
        p.task.abort();
        Ok(())
    }

    // Clone conn+state out before awaiting so the proxies map lock isn't held across await.
    async fn handle(
        &self,
        surface: &SurfaceId,
    ) -> Result<(Arc<DaemonConnection>, Arc<Mutex<ProxyState>>)> {
        let proxies = self.proxies.lock().await;
        let entry = proxies
            .get(surface)
            .ok_or_else(|| surface_err(surface, "no such surface"))?;
        Ok((entry.conn.clone(), entry.state.clone()))
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

    async fn spawn_terminal_proxy(
        &self,
        surface: SurfaceId,
        wire: WireSessionId,
        conn: Arc<DaemonConnection>,
        rx: tokio::sync::mpsc::Receiver<SessionFrame>,
        state: Arc<Mutex<ProxyState>>,
    ) {
        let ctx = self.ctx(surface.clone(), wire, conn.clone(), state.clone());
        let task = tokio::spawn(read_loop(ctx, rx));
        self.proxies
            .lock()
            .await
            .insert(surface, TerminalProxy { conn, state, task });
    }
}

fn wire_id(surface: &SurfaceId) -> WireSessionId {
    WireSessionId(surface.as_str().to_string())
}

fn surface_err(surface: &SurfaceId, reason: impl std::fmt::Display) -> OrchestratorError {
    OrchestratorError::Surface {
        surface: surface.as_str().to_string(),
        reason: reason.to_string(),
    }
}

async fn read_loop(ctx: ProxyCtx, mut rx: tokio::sync::mpsc::Receiver<SessionFrame>) {
    while let Some(frame) = rx.recv().await {
        if !handle_frame(&ctx, frame).await {
            break;
        }
    }
}

async fn handle_frame(ctx: &ProxyCtx, frame: SessionFrame) -> bool {
    match frame {
        SessionFrame::SpawnAck { .. } => {
            // Drain-and-set-Live atomically so input queued during Attaching isn't lost.
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
        SessionFrame::Error { .. } | SessionFrame::HelloAck { .. } | SessionFrame::Other { .. } => {
            true
        }
    }
}

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
    use daemon_pty_client::{encode_frame, FrameDecoder as DaemonFrameDecoder};
    use std::sync::Mutex as StdMutex;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{UnixListener, UnixStream};

    #[derive(Default)]
    struct CollectingSink {
        bytes: StdMutex<Vec<u8>>,
        statuses: StdMutex<Vec<String>>,
        exits: StdMutex<Vec<String>>,
        errors: StdMutex<Vec<String>>,
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
        fn on_error(&self, _surface: &SurfaceId, reason: &str) {
            self.errors.lock().unwrap().push(reason.to_string());
        }
    }

    async fn read_frame(
        rx: &mut tokio::net::unix::OwnedReadHalf,
        decoder: &mut DaemonFrameDecoder,
    ) -> RawFrameMeta {
        let mut buf = vec![0u8; 1024];
        loop {
            let n = rx.read(&mut buf).await.expect("read");
            assert!(n > 0, "socket closed");
            let frames = decoder.push(&buf[..n]);
            if let Some(f) = frames.into_iter().next() {
                let meta: serde_json::Value = serde_json::from_slice(&f.meta).expect("meta json");
                return RawFrameMeta {
                    ty: meta["type"].as_str().unwrap_or("").to_string(),
                };
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
    async fn launch_surface_streams_bytes_and_status_and_acks() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();

        let daemon = tokio::spawn(async move {
            let stream = accept(listener).await;
            let (mut rx, mut tx) = stream.into_split();
            let mut dec = DaemonFrameDecoder::new();
            assert_eq!(read_frame(&mut rx, &mut dec).await.ty, "hello");
            tx.write_all(&hello_ack()).await.unwrap();
            assert_eq!(read_frame(&mut rx, &mut dec).await.ty, "spawn");
            tx.write_all(&encode_frame(
                br#"{"type":"spawn-ack","sessionId":"surf-1","pid":7}"#,
                None,
            ))
            .await
            .unwrap();
            tx.write_all(&encode_frame(
                br#"{"type":"data","sessionId":"surf-1"}"#,
                Some(b"hi"),
            ))
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
            .launch_surface(
                SurfaceId::from_string("surf-1"),
                SurfaceKind::Terminal,
                None,
                "tok".into(),
                80,
                24,
                "/tmp".into(),
            )
            .await
            .expect("open");

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
            let mut dec = DaemonFrameDecoder::new();
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
            let mut dec = DaemonFrameDecoder::new();
            assert_eq!(read_frame(&mut rx, &mut dec).await.ty, "hello");
            tx.write_all(&hello_ack()).await.unwrap();
            assert_eq!(read_frame(&mut rx, &mut dec).await.ty, "spawn");
            // Delay the spawn-ack so the test's input() call is queued.
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            tx.write_all(&encode_frame(
                br#"{"type":"spawn-ack","sessionId":"s","pid":1}"#,
                None,
            ))
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
            .launch_surface(
                surface.clone(),
                SurfaceKind::Terminal,
                None,
                "t".into(),
                80,
                24,
                "/".into(),
            )
            .await
            .expect("open");
        // Send input while still attaching (spawn-ack delayed).
        runtime
            .input(&surface, b"ls\n")
            .await
            .expect("input queued");

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        daemon.abort();
    }

    async fn recording_daemon(listener: UnixListener, recorded: Arc<StdMutex<Vec<String>>>) {
        let Ok((stream, _)) = listener.accept().await else {
            return;
        };
        let (mut rx, mut tx) = stream.into_split();
        let mut buf = vec![0u8; 1024];
        let mut dec: DaemonFrameDecoder = DaemonFrameDecoder::new();
        loop {
            let Ok(n) = rx.read(&mut buf).await else {
                return;
            };
            if n == 0 {
                return;
            }
            for frame in dec.push(&buf[..n]) {
                let Ok(meta) = serde_json::from_slice::<serde_json::Value>(&frame.meta) else {
                    continue;
                };
                let ty = meta["type"].as_str().unwrap_or("").to_string();
                recorded.lock().unwrap().push(ty.clone());
                let id = meta["sessionId"].as_str().unwrap_or("").to_string();
                match ty.as_str() {
                    "hello" => {
                        let _ = tx.write_all(&hello_ack()).await;
                    }
                    "spawn" => {
                        let ack = format!(r#"{{"type":"spawn-ack","sessionId":"{id}","pid":1}}"#);
                        let _ = tx.write_all(&encode_frame(ack.as_bytes(), None)).await;
                    }
                    "subscribe" => {
                        let st = format!(
                            r#"{{"type":"status","sessionId":"{id}","status":"IDLE","source":"terminal"}}"#
                        );
                        let _ = tx.write_all(&encode_frame(st.as_bytes(), None)).await;
                    }
                    _ => {}
                }
            }
        }
    }

    async fn saw(recorded: &Arc<StdMutex<Vec<String>>>, ty: &str) -> bool {
        for _ in 0..50 {
            if recorded.lock().unwrap().iter().any(|t| t == ty) {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        false
    }

    fn recording_runtime(sock: std::path::PathBuf) -> (SurfaceRuntime, Arc<StdMutex<Vec<String>>>) {
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let sink = Arc::new(CollectingSink::default());
        (
            SurfaceRuntime::new(store, sink, sock),
            Arc::new(StdMutex::new(Vec::new())),
        )
    }

    #[tokio::test]
    async fn resize_forwards_a_resize_frame() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let (runtime, rec) = recording_runtime(sock);
        let daemon = tokio::spawn(recording_daemon(listener, rec.clone()));

        let surface = SurfaceId::from_string("rs");
        runtime
            .launch_surface(
                surface.clone(),
                SurfaceKind::Terminal,
                None,
                "t".into(),
                80,
                24,
                "/".into(),
            )
            .await
            .unwrap();
        runtime.resize(&surface, 100, 30).await.unwrap();

        assert!(saw(&rec, "resize").await);
        daemon.abort();
    }

    #[tokio::test]
    async fn detach_unsubscribes_and_drops_the_proxy() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let (runtime, rec) = recording_runtime(sock);
        let daemon = tokio::spawn(recording_daemon(listener, rec.clone()));

        let surface = SurfaceId::from_string("dt");
        runtime
            .launch_surface(
                surface.clone(),
                SurfaceKind::Terminal,
                None,
                "t".into(),
                80,
                24,
                "/".into(),
            )
            .await
            .unwrap();
        runtime.detach(&surface).await.unwrap();

        assert!(saw(&rec, "unsubscribe").await);
        assert_eq!(runtime.proxy_count().await, 0);
        daemon.abort();
    }

    #[tokio::test]
    async fn remove_kills_and_drops_the_proxy() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let (runtime, rec) = recording_runtime(sock);
        let daemon = tokio::spawn(recording_daemon(listener, rec.clone()));

        let surface = SurfaceId::from_string("rm");
        runtime
            .launch_surface(
                surface.clone(),
                SurfaceKind::Terminal,
                None,
                "t".into(),
                80,
                24,
                "/".into(),
            )
            .await
            .unwrap();
        runtime.remove(&surface).await.unwrap();

        assert!(saw(&rec, "kill").await);
        assert_eq!(runtime.proxy_count().await, 0);
        daemon.abort();
    }

    #[tokio::test]
    async fn resume_attaches_to_a_live_session() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let (runtime, rec) = recording_runtime(sock);
        let daemon = tokio::spawn(recording_daemon(listener, rec.clone()));

        runtime
            .resume(SurfaceId::from_string("live"))
            .await
            .expect("resume should attach to a live session");

        assert!(saw(&rec, "subscribe").await);
        assert_eq!(runtime.proxy_count().await, 1);
        daemon.abort();
    }

    async fn fake_spawn_daemon_capturing(
        listener: UnixListener,
        captured: Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
    ) {
        let Ok((stream, _)) = listener.accept().await else {
            return;
        };
        let (mut rx, mut tx) = stream.into_split();
        let mut buf = vec![0u8; 4096];
        let mut dec = DaemonFrameDecoder::new();
        while let Ok(n) = rx.read(&mut buf).await {
            if n == 0 {
                break;
            }
            for frame in dec.push(&buf[..n]) {
                let Ok(meta) = serde_json::from_slice::<serde_json::Value>(&frame.meta) else {
                    continue;
                };
                let id = meta["sessionId"].as_str().unwrap_or("").to_string();
                match meta["type"].as_str() {
                    Some("hello") => {
                        let _ = tx.write_all(&hello_ack()).await;
                    }
                    Some("spawn") => {
                        captured.lock().unwrap().push(meta.clone());
                        let ack = format!(r#"{{"type":"spawn-ack","sessionId":"{id}","pid":1}}"#);
                        let _ = tx.write_all(&encode_frame(ack.as_bytes(), None)).await;
                    }
                    _ => {}
                }
            }
        }
    }

    #[tokio::test]
    async fn generic_spawn_sends_resolved_command_to_daemon() {
        let dir = tempfile::tempdir().unwrap();
        let daemon_sock = dir.path().join("daemon.sock");
        let daemon_listener = UnixListener::bind(&daemon_sock).unwrap();
        let captured = Arc::new(std::sync::Mutex::new(Vec::new()));
        let _daemon = tokio::spawn(fake_spawn_daemon_capturing(
            daemon_listener,
            captured.clone(),
        ));

        let runtime = plain_runtime(daemon_sock, Arc::new(CollectingSink::default()));
        let cmd = ResolvedCommand {
            exe: "/bin/echo".into(),
            args: vec!["hi".into()],
            env: BTreeMap::new(),
        };
        runtime
            .spawn(
                &SurfaceId::from_string("s-1"),
                Some(&cmd),
                "/tmp",
                80,
                24,
                "tok",
            )
            .await
            .expect("spawn");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let spawns = captured.lock().unwrap().clone();
        assert_eq!(spawns.len(), 1, "expected one spawn frame");
        assert_eq!(spawns[0]["command"].as_str(), Some("/bin/echo"));
        let args: Vec<&str> = spawns[0]["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(args, vec!["hi"]);
    }

    #[tokio::test]
    async fn launch_surface_rejects_unsupported_kind() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = plain_runtime(
            dir.path().join("daemon.sock"),
            Arc::new(CollectingSink::default()),
        );
        let result = runtime
            .launch_surface(
                SurfaceId::from_string("d-1"),
                SurfaceKind::Diff,
                None,
                "t".into(),
                80,
                24,
                "/".into(),
            )
            .await;
        assert!(result.is_err(), "diff has no adapter; launch must error");
    }

    fn plain_runtime(daemon_sock: PathBuf, sink: Arc<CollectingSink>) -> SurfaceRuntime {
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        SurfaceRuntime::new(store, sink, daemon_sock)
    }
}
