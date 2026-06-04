//! Control plane: Unix-socket accept loop, per-connection framing, capability
//! negotiation, session registry, credit fan-out, and graceful shutdown.

use crate::codec::{encode_frame, FrameDecoder};
use crate::exit_qualifier::translate_exit;
use crate::hook_ingress::HookIngress;
use crate::manifest::{daemon_sock, hooks_sock, stopped_sessions_path, Manifest};
use crate::messages::{parse_client_frame, ClientFrame, SUPPORTED_VERSIONS};
use crate::pty_session::{Session, SessionEvent, INITIAL_CREDIT};
use crate::signals::SignalPlatform;
use crate::snapshot::{write_snapshot, SnapshotRecord};
use crate::stopped_sessions::StoppedSessionsStore;

use command_fds::{CommandFdExt, FdMapping};
use serde_json::json;
use std::collections::HashMap;
use std::os::fd::BorrowedFd;
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

pub const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Terminal-status sampling cadence and output-quiescence threshold. A session is
/// IDLE only after no output for `TERM_STATUS_QUIESCENCE`; the sampler ticks at
/// `TERM_STATUS_TICK` and emits a status frame only on transition.
const TERM_STATUS_TICK: Duration = Duration::from_millis(250);
const TERM_STATUS_QUIESCENCE: Duration = Duration::from_millis(400);

struct Connection {
    out_tx: UnboundedSender<Arc<[u8]>>,
    negotiated: bool,
    snapshot_capable: bool,
    hook_capable: bool,
}

pub struct State {
    pub sessions: HashMap<String, Session>,
    connections: HashMap<u64, Connection>,
    stopped: StoppedSessionsStore,
    next_conn_id: u64,
    events_tx: UnboundedSender<SessionEvent>,
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
    hooks_sock_path: PathBuf,
    hook_ingress: Arc<Mutex<Option<HookIngress>>>,
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
        };
        Daemon {
            state: Arc::new(Mutex::new(state)),
            sock_path: daemon_sock(dir),
            hooks_sock_path: hooks_sock(dir),
            hook_ingress: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_paths(mut self, sock: PathBuf, hooks: PathBuf) -> Self {
        self.sock_path = sock;
        self.hooks_sock_path = hooks;
        self
    }

    pub fn adopt_records(&self, records: &[SnapshotRecord]) -> usize {
        let mut st = self.state.lock().unwrap();
        let tx = st.events_tx.clone();
        let mut adopted = 0;
        for r in records {
            let session = Session::adopt(r, r.fd_index as RawFd, tx.clone());
            st.sessions.insert(session.session_id.clone(), session);
            adopted += 1;
        }
        adopted
    }

    pub async fn serve(
        &self,
        mut events_rx: tokio::sync::mpsc::UnboundedReceiver<SessionEvent>,
    ) -> std::io::Result<()> {
        let _ = std::fs::remove_file(&self.sock_path);
        let _ = std::fs::remove_file(&self.hooks_sock_path);
        let listener = UnixListener::bind(&self.sock_path)?;

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

        loop {
            let (stream, _) = listener.accept().await?;
            let daemon = self.clone();
            tokio::spawn(async move {
                daemon.handle_connection(stream).await;
            });
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
                    snapshot_capable: false,
                    hook_capable: false,
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
        let Some(ClientFrame::Hello {
            versions,
            capabilities,
        }) = parsed
        else {
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
        let caps = capabilities.unwrap_or_default();
        let hook_capable = caps.iter().any(|c| c == "hook");
        if let Some(c) = st.connections.get_mut(&conn_id) {
            c.snapshot_capable = caps.iter().any(|c| c == "snapshot");
            c.hook_capable = hook_capable;
            c.negotiated = true;
        }
        if hook_capable {
            self.ensure_hook_ingress();
        }
        let ack = encode_frame(
            &json!({
                "type": "hello-ack",
                "version": chosen,
                "daemonVersion": DAEMON_VERSION,
                "capabilities": ["snapshot", "hook"],
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
                let snapshot_capable = st
                    .connections
                    .get(&conn_id)
                    .is_some_and(|c| c.snapshot_capable);
                let Some(s) = st.sessions.get_mut(&session_id) else {
                    st.send_to(
                        conn_id,
                        error_frame("ENOTFOUND", "unknown session", Some(&session_id)),
                    );
                    return;
                };
                let term_status = s.term_status();
                if snapshot_capable {
                    let snap = s.build_snapshot();
                    s.add_subscriber(conn_id, INITIAL_CREDIT);
                    let frame = encode_frame(
                        &json!({
                            "type": "snapshot",
                            "sessionId": session_id,
                            "rows": snap.rows,
                            "cols": snap.cols,
                            "cells": snap.cells,
                            "cursor": snap.cursor,
                        }),
                        None,
                    );
                    st.send_to(conn_id, frame);
                } else {
                    let replay = s.replay_bytes();
                    let credit = (replay.len() as i64 + INITIAL_CREDIT).max(INITIAL_CREDIT);
                    s.add_subscriber(conn_id, credit);
                    if !replay.is_empty() {
                        st.send_to(conn_id, data_frame(&session_id, &replay));
                    }
                }
                st.send_to(conn_id, status_frame(&session_id, term_status.as_str(), "terminal"));
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

            ClientFrame::Upgrade => {
                let daemon = self.clone();
                tokio::spawn(async move { daemon.prepare_upgrade().await });
            }

            ClientFrame::Hello { .. } => {}
        }
    }

    fn ensure_hook_ingress(&self) {
        let mut guard = self.hook_ingress.lock().unwrap();
        if guard.is_some() {
            return;
        }
        let ingress = HookIngress::start(self.hooks_sock_path.clone(), self.clone());
        *guard = Some(ingress);
    }

    pub fn relay_hook(&self, session_id: &str, payload: serde_json::Value) {
        let st = self.state.lock().unwrap();
        let Some(s) = st.sessions.get(session_id) else {
            return;
        };
        let frame: Arc<[u8]> = encode_frame(
            &json!({ "type": "hook", "sessionId": session_id, "payload": payload }),
            None,
        )
        .into();
        for id in s.subscriber_ids() {
            let hook_capable = st.connections.get(&id).is_some_and(|c| c.hook_capable);
            if hook_capable {
                st.send_to(id, Arc::clone(&frame));
            }
        }
    }

    pub fn session_token(&self, session_id: &str) -> Option<String> {
        self.state
            .lock()
            .unwrap()
            .sessions
            .get(session_id)
            .map(|s| s.token.clone())
    }

    pub fn shutdown(&self) {
        let mut st = self.state.lock().unwrap();
        for s in st.sessions.values_mut() {
            s.force_kill_now();
        }
        st.sessions.clear();
        let _ = std::fs::remove_file(&self.sock_path);
        let _ = std::fs::remove_file(&self.hooks_sock_path);
    }

    // Sessions stay in the registry (never killed) so the master fds remain valid for dup.
    pub async fn prepare_upgrade(&self) {
        let (records, master_fds): (Vec<SnapshotRecord>, Vec<RawFd>) = {
            let st = self.state.lock().unwrap();
            let mut recs = Vec::new();
            let mut fds = Vec::new();
            for (i, (id, s)) in st.sessions.iter().enumerate() {
                let Some(fd) = s.raw_master_fd() else {
                    eprintln!("upgrade: no master fd for {id}; skipping");
                    continue;
                };
                let (cols, rows) = s.current_size();
                recs.push(SnapshotRecord {
                    session_id: id.clone(),
                    token: s.token.clone(),
                    pid: s.pid,
                    cols,
                    rows,
                    cwd: s.cwd.clone(),
                    fd_index: 4 + i as i32,
                    replay_buffer: SnapshotRecord::encode_replay(&s.replay_bytes()),
                });
                fds.push(fd);
            }
            (recs, fds)
        };

        if records.is_empty() {
            eprintln!("upgrade: no live sessions to hand off; staying up");
            return;
        }

        let dir = self
            .sock_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        let snap_path = dir.join("snapshot-upgrade.ndjson");
        if let Err(e) = write_snapshot(&snap_path, &records) {
            eprintln!("upgrade: snapshot write failed: {e}; staying up");
            return;
        }

        let exe = match std::env::current_exe() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("upgrade: cannot resolve own binary: {e}; staying up");
                return;
            }
        };

        // command-fds needs an OwnedFd per mapping; dup the session's fd so
        // command-fds closes the dup after spawn while the session keeps the original.
        let mut mappings: Vec<FdMapping> = Vec::with_capacity(master_fds.len());
        for (i, &fd) in master_fds.iter().enumerate() {
            // SAFETY: fd is the live session's PTY master, owned by a session
            // still in the registry (never killed on this path), so it is valid
            // for this borrow used only to immediately dup it.
            #[allow(unsafe_code)]
            let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
            match borrowed.try_clone_to_owned() {
                Ok(owned) => mappings.push(FdMapping {
                    parent_fd: owned,
                    child_fd: 4 + i as RawFd,
                }),
                Err(e) => {
                    eprintln!("upgrade: fd dup failed: {e}; staying up");
                    return;
                }
            }
        }

        let old_pid = std::process::id();
        let mut cmd = std::process::Command::new(&exe);
        cmd.arg("--handoff")
            .arg(format!("--snapshot={}", snap_path.display()))
            .arg(format!("--socket={}", self.sock_path.display()));
        if cmd.fd_mappings(mappings).is_err() {
            eprintln!("upgrade: fd mapping failed; staying up");
            return;
        }
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("upgrade: successor spawn failed: {e}; staying up");
                return;
            }
        };

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if Instant::now() >= deadline {
                eprintln!("upgrade: successor did not take over in time; aborting, staying up");
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
            if let Some(m) = Manifest::read(&dir) {
                if m.pid != old_pid {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        eprintln!(
            "upgrade: successor live; handing off {} session(s)",
            records.len()
        );
        let _ = std::fs::remove_file(&snap_path);
        self.stop_listening();
        std::process::exit(0);
    }

    fn stop_listening(&self) {
        // Drop hook ingress; leave control socket file — successor already rebound it.
        let _ = self.hook_ingress.lock().unwrap().take();
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
}
