//! Daemon-socket implementation of [`SurfaceRuntime`]: owns the per-surface PTY
//! proxies and the unix-socket transport. No persistence -- status flows out
//! through the [`SurfaceEventSink`]; the app layer records it.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use contracts::SessionId as WireSessionId;
use daemon_pty_client::{SessionFrame, SpawnParams};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use super::transport::DaemonConnection;
use super::{BoxFut, SpawnRequest, SurfaceEventSink, SurfaceRuntime};
use crate::entities::{SurfaceId, SurfaceKind};
use crate::shared::{Error, Result};

/// A fully resolved launch command: a concrete executable, its arguments, and
/// extra environment. `None` at a spawn site means the login shell.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedCommand {
    pub exe: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
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
    state: Arc<Mutex<ProxyState>>,
}

/// Drives PTYs in the daemon over a unix socket and proxies their output to a sink.
pub struct DaemonRuntime {
    sink: Arc<dyn SurfaceEventSink>,
    socket: PathBuf,
    proxies: Mutex<HashMap<SurfaceId, TerminalProxy>>,
}

impl DaemonRuntime {
    pub fn new(sink: Arc<dyn SurfaceEventSink>, socket: PathBuf) -> Self {
        Self {
            sink,
            socket,
            proxies: Mutex::new(HashMap::new()),
        }
    }

    pub async fn proxy_count(&self) -> usize {
        self.proxies.lock().await.len()
    }

    async fn launch(&self, request: SpawnRequest) -> Result<()> {
        let SpawnRequest {
            surface,
            kind,
            command,
            token,
            geometry,
            cwd,
        } = request;
        if self.proxies.lock().await.contains_key(&surface) {
            return Err(surface_err(&surface, "surface already has a proxy"));
        }
        if kind != SurfaceKind::Terminal {
            return Err(surface_err(
                &surface,
                format!("unsupported surface kind for launch: {}", kind.as_str()),
            ));
        }

        let wire = wire_id(&surface);
        let (conn, rx) = DaemonConnection::connect(&self.socket)
            .await
            .map_err(|e| surface_err(&surface, e))?;
        let conn = Arc::new(conn);

        const NO_ARGS: &[String] = &[];
        let params = SpawnParams {
            session_id: &wire,
            token: &token,
            cols: geometry.cols,
            rows: geometry.rows,
            cwd: &cwd,
            command: command.as_ref().map(|c| c.exe.as_str()),
            args: command
                .as_ref()
                .map(|c| c.args.as_slice())
                .unwrap_or(NO_ARGS),
            env: command.as_ref().map(|c| &c.env),
            resume: None,
        };
        conn.spawn(&params)
            .await
            .map_err(|e| surface_err(&surface, e))?;

        let state = Arc::new(Mutex::new(ProxyState::Attaching(Vec::new())));
        self.insert_proxy(surface, wire, conn, rx, state).await;
        Ok(())
    }

    async fn attach_proxy(&self, surface: &SurfaceId) -> Result<()> {
        if self.proxies.lock().await.contains_key(surface) {
            return Ok(());
        }

        let wire = wire_id(surface);
        let (conn, mut rx) = DaemonConnection::connect(&self.socket)
            .await
            .map_err(|e| surface_err(surface, e))?;
        let conn = Arc::new(conn);
        conn.subscribe(&wire)
            .await
            .map_err(|e| surface_err(surface, e))?;

        // subscribe replies with snapshot+status for a live session, or an error
        // frame for a missing one.
        let first = rx
            .recv()
            .await
            .ok_or_else(|| surface_err(surface, "daemon closed during attach"))?;
        if let SessionFrame::Error { code, message, .. } = &first {
            return Err(surface_err(
                surface,
                format!("session not resumable: {code} {message}"),
            ));
        }

        let state = Arc::new(Mutex::new(ProxyState::Live));
        let ctx = self.ctx(surface.clone(), wire.clone(), conn.clone(), state.clone());
        if !handle_frame(&ctx, first).await {
            return Err(surface_err(surface, "session exited during attach"));
        }

        let task = tokio::spawn(read_loop(ctx, rx));
        self.proxies
            .lock()
            .await
            .insert(surface.clone(), TerminalProxy { conn, state, task });
        Ok(())
    }

    async fn send_input(&self, surface: &SurfaceId, bytes: &[u8]) -> Result<()> {
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

    async fn send_resize(&self, surface: &SurfaceId, cols: u16, rows: u16) -> Result<()> {
        let (conn, _) = self.handle(surface).await?;
        conn.resize(&wire_id(surface), cols, rows)
            .await
            .map_err(|e| surface_err(surface, e))
    }

    async fn drop_proxy(&self, surface: &SurfaceId) -> Result<()> {
        let Some(p) = self.proxies.lock().await.remove(surface) else {
            return Ok(());
        };
        let _ = p.conn.unsubscribe(&wire_id(surface)).await;
        p.task.abort();
        Ok(())
    }

    async fn stop_surface(&self, surface: &SurfaceId) -> Result<()> {
        let proxy = self.proxies.lock().await.remove(surface);
        match proxy {
            Some(p) => {
                let _ = p.conn.stop(&wire_id(surface)).await;
                p.task.abort();
                Ok(())
            }
            None => {
                // No live proxy (already detached): open a one-shot connection to
                // deliver the stop so the daemon kills the PTY and suppresses resume.
                let (conn, _rx) = DaemonConnection::connect(&self.socket)
                    .await
                    .map_err(|e| surface_err(surface, e))?;
                conn.stop(&wire_id(surface))
                    .await
                    .map_err(|e| surface_err(surface, e))
            }
        }
    }

    async fn close_surface(&self, surface: &SurfaceId) -> Result<()> {
        let proxy = self.proxies.lock().await.remove(surface);
        match proxy {
            Some(p) => {
                let _ = p.conn.kill(&wire_id(surface)).await;
                p.task.abort();
                Ok(())
            }
            None => {
                let (conn, _rx) = DaemonConnection::connect(&self.socket)
                    .await
                    .map_err(|e| surface_err(surface, e))?;
                conn.kill(&wire_id(surface))
                    .await
                    .map_err(|e| surface_err(surface, e))
            }
        }
    }

    async fn list_running(&self) -> Result<Vec<SurfaceId>> {
        let ids = DaemonConnection::list_sessions(&self.socket)
            .await
            .map_err(|e| Error::SurfaceRuntime {
                surface: "*".to_string(),
                reason: e.to_string(),
            })?;
        Ok(ids.into_iter().map(SurfaceId::from_string).collect())
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
            state,
        }
    }

    async fn insert_proxy(
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

impl SurfaceRuntime for DaemonRuntime {
    fn spawn<'a>(&'a self, request: SpawnRequest) -> BoxFut<'a, ()> {
        Box::pin(self.launch(request))
    }

    fn stop<'a>(&'a self, surface: &'a SurfaceId) -> BoxFut<'a, ()> {
        Box::pin(self.stop_surface(surface))
    }

    fn close<'a>(&'a self, surface: &'a SurfaceId) -> BoxFut<'a, ()> {
        Box::pin(self.close_surface(surface))
    }

    fn list<'a>(&'a self) -> BoxFut<'a, Vec<SurfaceId>> {
        Box::pin(self.list_running())
    }

    fn input<'a>(&'a self, surface: &'a SurfaceId, bytes: &'a [u8]) -> BoxFut<'a, ()> {
        Box::pin(self.send_input(surface, bytes))
    }

    fn resize<'a>(&'a self, surface: &'a SurfaceId, cols: u16, rows: u16) -> BoxFut<'a, ()> {
        Box::pin(self.send_resize(surface, cols, rows))
    }

    fn attach<'a>(&'a self, surface: &'a SurfaceId) -> BoxFut<'a, ()> {
        Box::pin(self.attach_proxy(surface))
    }

    fn detach<'a>(&'a self, surface: &'a SurfaceId) -> BoxFut<'a, ()> {
        Box::pin(self.drop_proxy(surface))
    }
}

fn wire_id(surface: &SurfaceId) -> WireSessionId {
    WireSessionId(surface.as_str().to_string())
}

fn surface_err(surface: &SurfaceId, reason: impl std::fmt::Display) -> Error {
    Error::SurfaceRuntime {
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
            true
        }
        SessionFrame::Exit { qualifier, .. } => {
            ctx.sink.on_exit(&ctx.surface, &qualifier);
            *ctx.state.lock().await = ProxyState::Closed;
            false
        }
        SessionFrame::Error { message, .. } => {
            ctx.sink.on_error(&ctx.surface, &message);
            true
        }
        SessionFrame::ListAck { .. }
        | SessionFrame::HelloAck { .. }
        | SessionFrame::Other { .. } => true,
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
    use daemon_pty_client::{encode_frame, FrameDecoder as DaemonFrameDecoder};
    use std::sync::Mutex as StdMutex;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{UnixListener, UnixStream};

    use crate::infra::runtime::Geometry;

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

    fn spawn_request(surface: &str) -> SpawnRequest {
        SpawnRequest {
            surface: SurfaceId::from_string(surface),
            kind: SurfaceKind::Terminal,
            command: None,
            token: "tok".into(),
            geometry: Geometry { cols: 80, rows: 24 },
            cwd: "/tmp".into(),
        }
    }

    fn hello_ack() -> Vec<u8> {
        encode_frame(
            br#"{"type":"hello-ack","version":1,"daemonVersion":"0.0.0","capabilities":["snapshot"]}"#,
            None,
        )
    }

    async fn read_frame(
        rx: &mut tokio::net::unix::OwnedReadHalf,
        decoder: &mut DaemonFrameDecoder,
    ) -> String {
        let mut buf = vec![0u8; 1024];
        loop {
            let n = rx.read(&mut buf).await.expect("read");
            assert!(n > 0, "socket closed");
            if let Some(f) = decoder.push(&buf[..n]).into_iter().next() {
                let meta: serde_json::Value = serde_json::from_slice(&f.meta).expect("meta json");
                return meta["type"].as_str().unwrap_or("").to_string();
            }
        }
    }

    async fn accept(listener: UnixListener) -> UnixStream {
        let (stream, _) = listener.accept().await.expect("accept");
        stream
    }

    fn plain_runtime(sock: PathBuf) -> (DaemonRuntime, Arc<CollectingSink>) {
        let sink = Arc::new(CollectingSink::default());
        (DaemonRuntime::new(sink.clone(), sock), sink)
    }

    #[tokio::test]
    async fn spawn_streams_bytes_and_status_and_acks() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();

        let daemon = tokio::spawn(async move {
            let stream = accept(listener).await;
            let (mut rx, mut tx) = stream.into_split();
            let mut dec = DaemonFrameDecoder::new();
            assert_eq!(read_frame(&mut rx, &mut dec).await, "hello");
            tx.write_all(&hello_ack()).await.unwrap();
            assert_eq!(read_frame(&mut rx, &mut dec).await, "spawn");
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
            assert_eq!(read_frame(&mut rx, &mut dec).await, "ack");
            tx.write_all(&encode_frame(
                br#"{"type":"status","sessionId":"surf-1","status":"WORKING","source":"terminal"}"#,
                None,
            ))
            .await
            .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        });

        let (runtime, sink) = plain_runtime(sock);
        runtime.spawn(spawn_request("surf-1")).await.expect("spawn");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(runtime.proxy_count().await, 1);
        assert_eq!(&*sink.bytes.lock().unwrap(), b"hi");
        assert_eq!(&*sink.statuses.lock().unwrap(), &["WORKING".to_string()]);
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

    fn recording_runtime(sock: PathBuf) -> (DaemonRuntime, Arc<StdMutex<Vec<String>>>) {
        let sink = Arc::new(CollectingSink::default());
        (
            DaemonRuntime::new(sink, sock),
            Arc::new(StdMutex::new(Vec::new())),
        )
    }

    async fn launched_runtime(
        sock: PathBuf,
        surface: &str,
    ) -> (DaemonRuntime, Arc<StdMutex<Vec<String>>>, SurfaceId) {
        let listener = UnixListener::bind(&sock).unwrap();
        let (runtime, rec) = recording_runtime(sock);
        tokio::spawn(recording_daemon(listener, rec.clone()));
        let id = SurfaceId::from_string(surface);
        runtime.spawn(spawn_request(surface)).await.unwrap();
        (runtime, rec, id)
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
            assert_eq!(read_frame(&mut rx, &mut dec).await, "hello");
            tx.write_all(&hello_ack()).await.unwrap();
            assert_eq!(read_frame(&mut rx, &mut dec).await, "spawn");
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            tx.write_all(&encode_frame(
                br#"{"type":"spawn-ack","sessionId":"s","pid":1}"#,
                None,
            ))
            .await
            .unwrap();
            assert_eq!(read_frame(&mut rx, &mut dec).await, "input");
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        });

        let (runtime, _sink) = plain_runtime(sock);
        let surface = SurfaceId::from_string("s");
        runtime.spawn(spawn_request("s")).await.expect("spawn");
        runtime
            .input(&surface, b"ls\n")
            .await
            .expect("input queued");

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        daemon.abort();
    }

    #[tokio::test]
    async fn resize_forwards_a_resize_frame() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let (runtime, rec, surface) = launched_runtime(sock, "rs").await;

        runtime.resize(&surface, 100, 30).await.unwrap();

        assert!(saw(&rec, "resize").await);
    }

    #[tokio::test]
    async fn detach_unsubscribes_and_drops_the_proxy() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let (runtime, rec, surface) = launched_runtime(sock, "dt").await;

        runtime.detach(&surface).await.unwrap();

        assert!(saw(&rec, "unsubscribe").await);
        assert_eq!(runtime.proxy_count().await, 0);
    }

    #[tokio::test]
    async fn stop_sends_a_stop_frame_and_drops_the_proxy() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let (runtime, rec, surface) = launched_runtime(sock, "sp").await;

        runtime.stop(&surface).await.unwrap();

        assert!(saw(&rec, "stop").await);
        assert_eq!(runtime.proxy_count().await, 0);
    }

    #[tokio::test]
    async fn close_kills_and_drops_the_proxy() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let (runtime, rec, surface) = launched_runtime(sock, "cl").await;

        runtime.close(&surface).await.unwrap();

        assert!(saw(&rec, "kill").await);
        assert_eq!(runtime.proxy_count().await, 0);
    }

    #[tokio::test]
    async fn attach_subscribes_to_a_live_session() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let (runtime, rec) = recording_runtime(sock);
        tokio::spawn(recording_daemon(listener, rec.clone()));

        runtime
            .attach(&SurfaceId::from_string("live"))
            .await
            .expect("attach should subscribe to a live session");

        assert!(saw(&rec, "subscribe").await);
        assert_eq!(runtime.proxy_count().await, 1);
    }

    #[tokio::test]
    async fn list_returns_the_daemon_running_surface_ids() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();

        tokio::spawn(async move {
            let stream = accept(listener).await;
            let (mut rx, mut tx) = stream.into_split();
            let mut dec = DaemonFrameDecoder::new();
            assert_eq!(read_frame(&mut rx, &mut dec).await, "hello");
            tx.write_all(&hello_ack()).await.unwrap();
            assert_eq!(read_frame(&mut rx, &mut dec).await, "list");
            tx.write_all(&encode_frame(
                br#"{"type":"list-ack","ids":["surf-x","surf-y"]}"#,
                None,
            ))
            .await
            .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        });

        let (runtime, _sink) = plain_runtime(sock);
        let ids = runtime.list().await.expect("list");
        assert_eq!(
            ids,
            vec![
                SurfaceId::from_string("surf-x"),
                SurfaceId::from_string("surf-y"),
            ]
        );
    }

    #[tokio::test]
    async fn spawn_rejects_unsupported_kind() {
        let dir = tempfile::tempdir().unwrap();
        let (runtime, _sink) = plain_runtime(dir.path().join("daemon.sock"));
        let mut request = spawn_request("d-1");
        request.kind = SurfaceKind::Diff;
        let result = runtime.spawn(request).await;
        assert!(result.is_err(), "diff has no adapter; spawn must error");
    }
}
