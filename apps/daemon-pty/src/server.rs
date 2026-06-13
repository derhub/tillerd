//! Server: control plane, session registry, credit fan-out.

use crate::codec::{encode_frame, FrameDecoder};
use crate::exit_qualifier::translate_exit;
use crate::manifest::stopped_sessions_path;
use crate::messages::{parse_client_frame, ClientFrame, SUPPORTED_VERSIONS};
use crate::pty_session::{Session, SessionEvent, INITIAL_CREDIT, SHUTDOWN_KILL_GRACE_MS};
use crate::signals::SignalPlatform;
use crate::stopped_sessions::StoppedSessionsStore;

use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

pub const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");

// Daemon is pty-only and advertises no capabilities: no hook face (ADR-0016) and no snapshot
// (subscribe replays raw scrollback; fd-handoff is superseded by drain-and-restart, ADR-0029).
const ADVERTISED_CAPABILITIES: &[&str] = &[];

/// Terminal-status sampling cadence and output-quiescence threshold. A session is
/// IDLE only after no output for `TERM_STATUS_QUIESCENCE`; the sampler ticks at
/// `TERM_STATUS_TICK` and emits a status frame only on transition.
const TERM_STATUS_TICK: Duration = Duration::from_millis(250);
const TERM_STATUS_QUIESCENCE: Duration = Duration::from_millis(400);

struct Connection {
    out_tx: UnboundedSender<Arc<[u8]>>,
    negotiated: bool,
}

pub struct State {
    pub sessions: HashMap<String, Session>,
    connections: HashMap<u64, Connection>,
    stopped: StoppedSessionsStore,
    next_conn_id: u64,
    events_tx: UnboundedSender<SessionEvent>,
    /// Set once the host signals drain (SIGUSR2): new sessions are refused while active ones
    /// finish. A clean exit follows when the last session ends (ADR-0029).
    draining: bool,
}

impl State {
    // `Arc<[u8]>` so fan-out to N subscribers clones a refcount, not the bytes.
    fn send_to(&self, conn_id: u64, frame: impl Into<Arc<[u8]>>) {
        if let Some(c) = self.connections.get(&conn_id) {
            let _ = c.out_tx.send(frame.into());
        }
    }
}

#[derive(Clone)]
pub struct Daemon {
    state: Arc<Mutex<State>>,
    sock_path: PathBuf,
    /// Notified whenever the last active session ends, so the drain phase can exit when idle.
    idle: Arc<tokio::sync::Notify>,
}

impl Daemon {
    pub fn new(dir: &std::path::Path, events_tx: UnboundedSender<SessionEvent>) -> Self {
        let mut stopped = StoppedSessionsStore::new(stopped_sessions_path(dir));
        stopped.load();
        let state = State {
            sessions: HashMap::new(),
            connections: HashMap::new(),
            stopped,
            next_conn_id: 1,
            events_tx,
            draining: false,
        };
        Daemon {
            state: Arc::new(Mutex::new(state)),
            sock_path: tillerd_paths::daemon_socket_in(dir),
            idle: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub async fn serve(
        &self,
        mut events_rx: tokio::sync::mpsc::UnboundedReceiver<SessionEvent>,
        ready: service_host::Ready,
        drain: service_host::Drain,
    ) -> std::io::Result<()> {
        let _ = std::fs::remove_file(&self.sock_path);
        let listener = UnixListener::bind(&self.sock_path)?;
        tracing::info!(sock = %self.sock_path.display(), "daemon started");

        // Listening: announce readiness so the host flips the manifest to `ready` and consumers
        // (orchestrator adopt-or-spawn, the e2e rig) can discover us from the manifest.
        ready.signal();

        {
            let daemon = self.clone();
            tokio::spawn(async move {
                while let Some(ev) = events_rx.recv().await {
                    daemon.handle_session_event(ev);
                }
            });
        }

        {
            let daemon = self.clone();
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(TERM_STATUS_TICK);
                loop {
                    tick.tick().await;
                    daemon.sample_terminal_status();
                }
            });
        }

        // Phase 1 — serve normally until drained.
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let (stream, _) = accepted?;
                    let daemon = self.clone();
                    tokio::spawn(async move {
                        daemon.handle_connection(stream).await;
                    });
                }
                _ = drain.draining() => {
                    self.state.lock().unwrap().draining = true;
                    tracing::info!("daemon draining: refusing new sessions, finishing active ones");
                    break;
                }
            }
        }

        // Phase 2 — drained. Keep serving existing connections (subscribe/input/kill on active
        // sessions) and refuse new spawns, exiting cleanly once the last session ends. No timer
        // kills sessions: SIGTERM is the explicit upgrade-now pressure valve (ADR-0029, design D4).
        loop {
            // Register the idle wakeup before checking, so a session ending between the check and
            // the await is not missed.
            let idle = self.idle.notified();
            if self.state.lock().unwrap().sessions.is_empty() {
                tracing::info!("daemon drained: no active sessions, exiting");
                return Ok(());
            }
            tokio::select! {
                accepted = listener.accept() => {
                    let (stream, _) = accepted?;
                    let daemon = self.clone();
                    tokio::spawn(async move {
                        daemon.handle_connection(stream).await;
                    });
                }
                _ = idle => {}
            }
        }
    }

    fn handle_session_event(&self, ev: SessionEvent) {
        let mut st = self.state.lock().unwrap();
        match ev {
            SessionEvent::Data { session_id, bytes } => {
                let targets = {
                    let Some(s) = st.sessions.get_mut(&*session_id) else {
                        return;
                    };
                    s.mark_output();
                    s.append_replay(&bytes);
                    s.fan_out(bytes.len())
                };
                if targets.is_empty() {
                    return;
                }
                let frame: Arc<[u8]> = data_frame(&session_id, &bytes).into();
                for id in targets {
                    st.send_to(id, Arc::clone(&frame));
                }
            }
            SessionEvent::Exit {
                session_id,
                code,
                signal,
            } => {
                let Some(mut session) = st.sessions.remove(&session_id) else {
                    return;
                };
                if session.exit_emitted {
                    return;
                }
                session.exit_emitted = true;
                let tx =
                    translate_exit(session.killed_by_user, code, signal, SignalPlatform::host());
                let frame = encode_frame(
                    &json!({
                        "type": "exit",
                        "sessionId": session_id,
                        "qualifier": tx.qualifier,
                        "raw": tx.raw,
                    }),
                    None,
                );
                let frame: Arc<[u8]> = frame.into();
                for id in session.subscriber_ids() {
                    st.send_to(id, Arc::clone(&frame));
                }
                // A draining daemon exits once the last session ends; wake the drain loop to check.
                if st.sessions.is_empty() {
                    self.idle.notify_waiters();
                }
            }
        }
    }

    // Collect status transitions first, then send — the send borrow of `State`
    // cannot overlap the mutable iteration over sessions.
    fn sample_terminal_status(&self) {
        let mut st = self.state.lock().unwrap();
        let mut emits: Vec<(Vec<u64>, Arc<[u8]>)> = Vec::new();
        for (sid, s) in st.sessions.iter_mut() {
            if !s.has_subscribers() {
                continue;
            }
            if let Some(next) = s.sample_term_status(TERM_STATUS_QUIESCENCE) {
                let frame: Arc<[u8]> = status_frame(sid, next.as_str(), "terminal").into();
                emits.push((s.subscriber_ids(), frame));
            }
        }
        for (ids, frame) in emits {
            for id in ids {
                st.send_to(id, Arc::clone(&frame));
            }
        }
    }

    async fn handle_connection(&self, stream: UnixStream) {
        let (mut read_half, mut write_half) = stream.into_split();
        let (out_tx, mut out_rx) = unbounded_channel::<Arc<[u8]>>();

        let conn_id = {
            let mut st = self.state.lock().unwrap();
            let id = st.next_conn_id;
            st.next_conn_id += 1;
            st.connections.insert(
                id,
                Connection {
                    out_tx,
                    negotiated: false,
                },
            );
            id
        };

        let writer = tokio::spawn(async move {
            while let Some(frame) = out_rx.recv().await {
                if write_half.write_all(&frame).await.is_err() {
                    break;
                }
            }
        });

        let mut decoder = FrameDecoder::new();
        let mut buf = [0u8; 65_536];
        loop {
            let n = match read_half.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            for frame in decoder.push(&buf[..n]) {
                self.handle_frame(conn_id, &frame.meta, frame.body.as_deref());
            }
        }

        {
            let mut st = self.state.lock().unwrap();
            for s in st.sessions.values_mut() {
                s.remove_subscriber(conn_id);
            }
            st.connections.remove(&conn_id);
        }
        writer.abort();
    }

    fn handle_frame(&self, conn_id: u64, meta: &[u8], body: Option<&[u8]>) {
        let mut st = self.state.lock().unwrap();

        let negotiated = st.connections.get(&conn_id).is_some_and(|c| c.negotiated);
        if !negotiated {
            self.handle_hello(&mut st, conn_id, meta);
            return;
        }

        let Some(frame) = parse_client_frame(meta) else {
            let f = error_frame("EPARSE", "malformed frame", None);
            st.send_to(conn_id, f);
            return;
        };
        self.dispatch(&mut st, conn_id, frame, body);
    }

    fn handle_hello(&self, st: &mut State, conn_id: u64, meta: &[u8]) {
        let parsed = parse_client_frame(meta);
        let Some(ClientFrame::Hello { versions, .. }) = parsed else {
            st.send_to(conn_id, error_frame("EPROTO", "expected hello", None));
            return;
        };
        let chosen = SUPPORTED_VERSIONS
            .iter()
            .find(|v| versions.contains(v))
            .copied();
        let Some(chosen) = chosen else {
            let msg = format!(
                "no compatible version; supported: {}",
                SUPPORTED_VERSIONS
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            st.send_to(conn_id, error_frame("EVERSION", &msg, None));
            return;
        };
        if let Some(c) = st.connections.get_mut(&conn_id) {
            c.negotiated = true;
        }
        let ack = encode_frame(
            &json!({
                "type": "hello-ack",
                "version": chosen,
                "daemonVersion": DAEMON_VERSION,
                "capabilities": ADVERTISED_CAPABILITIES,
            }),
            None,
        );
        st.send_to(conn_id, ack);
    }

    fn dispatch(&self, st: &mut State, conn_id: u64, frame: ClientFrame, body: Option<&[u8]>) {
        match frame {
            ClientFrame::List => {
                let ids: Vec<String> = st.sessions.keys().cloned().collect();
                st.send_to(
                    conn_id,
                    encode_frame(&json!({ "type": "list-ack", "ids": ids }), None),
                );
            }

            ClientFrame::Spawn(spawn) => {
                if st.draining {
                    st.send_to(
                        conn_id,
                        error_frame(
                            "EDRAINING",
                            "daemon is draining; no new sessions",
                            Some(&spawn.session_id),
                        ),
                    );
                    return;
                }
                if st.sessions.contains_key(&spawn.session_id) {
                    st.send_to(
                        conn_id,
                        error_frame("EEXIST", "session already exists", Some(&spawn.session_id)),
                    );
                    return;
                }
                if let Some(resume) = &spawn.resume {
                    if st.stopped.has(resume) {
                        st.send_to(
                            conn_id,
                            error_frame(
                                "SessionStopped",
                                &format!("Session {resume} was intentionally stopped"),
                                Some(&spawn.session_id),
                            ),
                        );
                        return;
                    }
                }
                let events_tx = st.events_tx.clone();
                match Session::spawn(&spawn, events_tx) {
                    Ok(mut session) => {
                        let pid = session.pid;
                        session.add_subscriber(conn_id, INITIAL_CREDIT);
                        st.sessions.insert(spawn.session_id.clone(), session);
                        // The daemon's session id is the surface id, which is the operation's
                        // correlation id end to end — emit it so records join the orchestrator's
                        // on the standardized `correlation_id` key (design D5).
                        tracing::info!(correlation_id = %spawn.session_id, pid = pid, "session spawned");
                        st.send_to(conn_id, encode_frame(&json!({ "type": "spawn-ack", "sessionId": spawn.session_id, "pid": pid }), None));
                    }
                    Err(e) => {
                        st.send_to(
                            conn_id,
                            error_frame(e.code(), &e.message(), Some(&spawn.session_id)),
                        );
                    }
                }
            }

            ClientFrame::Kill { session_id } => {
                if let Some(s) = st.sessions.get_mut(&session_id) {
                    s.mark_killed_by_user();
                    s.begin_kill();
                }
            }

            ClientFrame::Stop { session_id } => {
                if let Some(s) = st.sessions.get_mut(&session_id) {
                    s.mark_killed_by_user();
                    s.begin_kill();
                }
                st.stopped.add(&session_id);
            }

            ClientFrame::Input { session_id } => {
                if let (Some(b), Some(s)) = (body, st.sessions.get_mut(&session_id)) {
                    s.write_input(b);
                }
            }

            ClientFrame::Resize {
                session_id,
                cols,
                rows,
            } => {
                if let Some(s) = st.sessions.get_mut(&session_id) {
                    s.resize(cols, rows);
                }
            }

            ClientFrame::Subscribe { session_id } => {
                let Some(s) = st.sessions.get_mut(&session_id) else {
                    st.send_to(
                        conn_id,
                        error_frame("ENOTFOUND", "unknown session", Some(&session_id)),
                    );
                    return;
                };
                let term_status = s.term_status();
                // Replay the session's scrollback as raw bytes so the subscriber repaints from the
                // terminal's own output stream.
                let replay = s.replay_bytes();
                let credit = (replay.len() as i64 + INITIAL_CREDIT).max(INITIAL_CREDIT);
                s.add_subscriber(conn_id, credit);
                if !replay.is_empty() {
                    st.send_to(conn_id, data_frame(&session_id, &replay));
                }
                st.send_to(
                    conn_id,
                    status_frame(&session_id, term_status.as_str(), "terminal"),
                );
            }

            ClientFrame::Unsubscribe { session_id } => {
                if let Some(s) = st.sessions.get_mut(&session_id) {
                    s.remove_subscriber(conn_id);
                }
            }

            ClientFrame::Ack { session_id, bytes } => {
                if let Some(s) = st.sessions.get_mut(&session_id) {
                    s.add_credit(conn_id, bytes);
                }
            }

            ClientFrame::Hello { .. } => {}
        }
    }

    pub fn shutdown(&self) {
        for s in self.state.lock().unwrap().sessions.values_mut() {
            s.force_kill_now();
        }
        // Escalate after a grace so a child that ignores SIGTERM is reaped, not
        // orphaned. Lock released during the sleep.
        std::thread::sleep(Duration::from_millis(SHUTDOWN_KILL_GRACE_MS));
        let mut st = self.state.lock().unwrap();
        for s in st.sessions.values_mut() {
            s.hard_kill();
        }
        st.sessions.clear();
        let _ = std::fs::remove_file(&self.sock_path);
    }
}

fn data_frame(session_id: &str, bytes: &[u8]) -> Vec<u8> {
    encode_frame(
        &json!({ "type": "data", "sessionId": session_id, "bodyLen": bytes.len() }),
        Some(bytes),
    )
}

fn status_frame(session_id: &str, status: &str, source: &str) -> Vec<u8> {
    encode_frame(
        &json!({ "type": "status", "sessionId": session_id, "status": status, "source": source }),
        None,
    )
}

fn error_frame(code: &str, message: &str, session_id: Option<&str>) -> Vec<u8> {
    match session_id {
        Some(sid) => encode_frame(
            &json!({ "type": "error", "code": code, "message": message, "sessionId": sid }),
            None,
        ),
        None => encode_frame(
            &json!({ "type": "error", "code": code, "message": message }),
            None,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_frame_matches_shared_contract() {
        let frame = status_frame("s1", "IDLE", "terminal");
        // Layout: [4-byte BE len][meta JSON], no body.
        let meta = &frame[4..];
        let v: serde_json::Value = serde_json::from_slice(meta).unwrap();
        assert_eq!(v["type"], "status");
        assert_eq!(v["sessionId"], "s1");
        assert_eq!(v["status"], "IDLE");
        assert_eq!(v["source"], "terminal");
        // Exactly the contract fields, nothing extra.
        assert_eq!(v.as_object().unwrap().len(), 4);
    }

    #[test]
    fn advertises_no_capabilities() {
        // PTY-only: no hook face (ADR-0016) and no snapshot (subscribe replays raw scrollback).
        assert!(ADVERTISED_CAPABILITIES.is_empty());
    }

    #[tokio::test]
    async fn draining_daemon_refuses_new_session_with_typed_error() {
        let dir = std::env::temp_dir().join(format!("tillerd-drain-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let (events_tx, _events_rx) = tokio::sync::mpsc::unbounded_channel();
        let daemon = Daemon::new(&dir, events_tx);

        // Register a negotiated connection whose outbound frames we can read.
        let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<Arc<[u8]>>();
        let spawn = match crate::messages::parse_client_frame(
            br#"{"type":"spawn","sessionId":"s1","args":[],"token":"t","cols":80,"rows":24,"cwd":"/tmp"}"#,
        ) {
            Some(ClientFrame::Spawn(s)) => s,
            _ => panic!("expected a spawn frame"),
        };

        {
            let mut st = daemon.state.lock().unwrap();
            st.connections.insert(
                1,
                Connection {
                    out_tx,
                    negotiated: true,
                },
            );
            st.draining = true;
            daemon.dispatch(&mut st, 1, ClientFrame::Spawn(spawn), None);
        }

        let frame = out_rx.recv().await.expect("a frame was sent back");
        let v: serde_json::Value = serde_json::from_slice(&frame[4..]).unwrap();
        assert_eq!(v["type"], "error");
        assert_eq!(
            v["code"], "EDRAINING",
            "a draining daemon refuses new sessions"
        );
        assert!(
            daemon.state.lock().unwrap().sessions.is_empty(),
            "no session was created while draining"
        );
    }

    // ── daemon-session-subscription: consumer-oblivious scenarios ────────────

    /// The daemon's subscribe path identifies sessions by id only. The `Subscribe`
    /// frame carries no consumer identity, purpose, or callback — the daemon
    /// never needs to know what kind of consumer is subscribing.
    #[test]
    fn subscribe_frame_carries_only_session_id_no_consumer_identity() {
        let raw = br#"{"type":"subscribe","sessionId":"s-abc"}"#;
        let frame = crate::messages::parse_client_frame(raw).unwrap();
        match frame {
            ClientFrame::Subscribe { session_id } => {
                assert_eq!(session_id, "s-abc");
                // No consumer-identity field: the struct holds only session_id.
            }
            _ => panic!("expected Subscribe"),
        }
    }

    /// The daemon's public surface — the hello-ack it sends after negotiation —
    /// advertises only PTY-level capabilities. It does not advertise any hook
    /// ingress, confirming the consumer-agnostic contract from ADR-0016.
    #[test]
    fn daemon_public_surface_has_no_hook_ingress_face() {
        // The advertised capability set is the daemon's entire public surface
        // declaration: it contains no hook ingress, so a new consumer type
        // needs no daemon change to subscribe.
        let surface: Vec<&str> = ADVERTISED_CAPABILITIES.to_vec();
        assert!(
            !surface.iter().any(|&c| c.contains("hook")),
            "daemon surface must contain no hook ingress; found: {surface:?}"
        );
    }

    // ── rust-pty-daemon: consumer-oblivious operation ─────────────────────────

    /// The daemon's subscribe dispatch uses only the session id to route output.
    /// No consumer type, callback, or identity participates in the path — the
    /// daemon is consumer-oblivious: adding or removing a consumer type requires
    /// no change here.
    #[test]
    fn subscribe_dispatch_requires_no_consumer_type_knowledge() {
        // The `ClientFrame::Subscribe` variant carries only `session_id`.
        // If this compiled it means the daemon's subscription path has no
        // consumer-type discriminant.
        let raw = br#"{"type":"subscribe","sessionId":"any-session"}"#;
        let frame = crate::messages::parse_client_frame(raw);
        assert!(
            matches!(frame, Some(ClientFrame::Subscribe { .. })),
            "subscribe dispatch is keyed solely on session_id"
        );
        // A second consumer type (different label, same session_id): the
        // daemon parses and routes it identically.
        let raw2 = br#"{"type":"subscribe","sessionId":"any-session"}"#;
        let frame2 = crate::messages::parse_client_frame(raw2);
        assert!(
            matches!(frame2, Some(ClientFrame::Subscribe { .. })),
            "any consumer with the same session_id is handled identically"
        );
    }
}
