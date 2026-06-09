//! Hook-event source: stub (replay) or gate subscription.
//! [`gate_client`]. The engine stays synchronous: blocking reads on a dedicated
//! capture thread, no async runtime.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;

use anyhow::Context;
use contracts::{HookEvent, SessionId};
use gate_client::{
    decode_subscription_frame, encode_subscribe_preamble, negotiate_ready, FrameDecoder,
    SubscriptionFrame,
};

const READ_BUF: usize = 8192;

/// A pull source of normalized lifecycle events. `next` blocks until an event is
/// available and returns `None` once the source is exhausted or closed.
pub trait HookSource: Send {
    /// The next event, or `None` at end of stream.
    fn next(&mut self) -> Option<HookEvent>;
}

/// Replays a fixed list of events, then yields `None`. Never blocks.
pub struct StubSource {
    events: std::vec::IntoIter<HookEvent>,
}

impl StubSource {
    /// A source that yields `events` in order.
    pub fn new(events: Vec<HookEvent>) -> Self {
        Self {
            events: events.into_iter(),
        }
    }
}

impl HookSource for StubSource {
    fn next(&mut self) -> Option<HookEvent> {
        self.events.next()
    }
}

/// Subscribes to a gate's hook-event stream over a blocking Unix socket.
#[derive(Debug)]
pub struct GateSubscriptionSource {
    stream: UnixStream,
    decoder: FrameDecoder,
    pending: VecDeque<HookEvent>,
}

impl GateSubscriptionSource {
    /// Connect to the gate at `socket_path`, subscribe to `session_id`, and
    /// negotiate the wire version. Events delivered alongside the ready
    /// handshake are buffered.
    pub fn connect(socket_path: impl AsRef<Path>, session_id: SessionId) -> anyhow::Result<Self> {
        let mut stream = UnixStream::connect(socket_path)?;
        stream.write_all(&encode_subscribe_preamble(&session_id))?;
        stream.flush()?;

        let mut decoder = FrameDecoder::new();
        let mut pending = VecDeque::new();
        let mut buf = [0u8; READ_BUF];
        loop {
            let n = stream.read(&mut buf)?;
            if n == 0 {
                anyhow::bail!("gate closed the connection before the ready handshake");
            }
            let mut frames = decoder.push(&buf[..n])?.into_iter();
            let Some(first) = frames.next() else {
                continue;
            };
            let ready =
                decode_subscription_frame(&first).context("gate sent a malformed ready frame")?;
            negotiate_ready(&ready)?;
            for raw in frames {
                if let Some(SubscriptionFrame::Event(event)) = decode_subscription_frame(&raw) {
                    pending.push_back(event);
                }
            }
            return Ok(Self {
                stream,
                decoder,
                pending,
            });
        }
    }
}

impl HookSource for GateSubscriptionSource {
    fn next(&mut self) -> Option<HookEvent> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Some(event);
            }
            let mut buf = [0u8; READ_BUF];
            let n = self.stream.read(&mut buf).ok()?;
            if n == 0 {
                return None;
            }
            let frames = self.decoder.push(&buf[..n]).ok()?;
            for raw in frames {
                if let Some(SubscriptionFrame::Event(event)) = decode_subscription_frame(&raw) {
                    self.pending.push_back(event);
                }
            }
        }
    }
}

impl Drop for GateSubscriptionSource {
    fn drop(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::HookCapturer;
    use crate::Engram;
    use contracts::{CorrelationId, HookKind};
    use gate_client::{encode_frame, WIRE_VERSION};
    use serde_json::json;
    use std::os::unix::net::UnixListener;
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    fn temp_sock(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        // A short base: a Unix socket path must fit in `SUN_LEN` (~104 on macOS).
        std::path::PathBuf::from(format!("/tmp/eg-{tag}-{}-{nanos}.sock", std::process::id()))
    }

    fn ready_frame(wire_version: u32) -> Vec<u8> {
        encode_frame(
            &serde_json::to_vec(&json!({ "frame": "ready", "wireVersion": wire_version })).unwrap(),
        )
    }

    fn event_frame(event: &HookEvent) -> Vec<u8> {
        encode_frame(&serde_json::to_vec(&json!({ "frame": "event", "event": event })).unwrap())
    }

    fn prompt_event(correlation: &str, content: &str) -> HookEvent {
        HookEvent {
            session_id: SessionId("s1".into()),
            correlation_id: CorrelationId(correlation.into()),
            ts: 10,
            kind: HookKind::UserPromptSubmit {
                content: content.into(),
                turn_index: Some(0),
            },
        }
    }

    #[test]
    fn stub_source_yields_events_in_order_then_none() {
        let events = vec![prompt_event("c1", "one"), prompt_event("c2", "two")];
        let mut source = StubSource::new(events.clone());

        assert_eq!(source.next(), Some(events[0].clone()));
        assert_eq!(source.next(), Some(events[1].clone()));
        assert_eq!(source.next(), None);
    }

    #[test]
    fn stub_source_next_never_blocks() {
        let mut source = StubSource::new(vec![]);
        assert_eq!(source.next(), None);
    }

    #[test]
    fn gate_subscription_source_decodes_events_from_mock_unix_socket() {
        let sock = temp_sock("decode");
        let listener = UnixListener::bind(&sock).unwrap();
        let events = vec![prompt_event("c1", "first"), prompt_event("c2", "second")];

        let server_events = events.clone();
        let server = thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = conn.read(&mut buf).unwrap();
            conn.write_all(&ready_frame(WIRE_VERSION)).unwrap();
            for e in &server_events {
                conn.write_all(&event_frame(e)).unwrap();
            }
            conn.flush().unwrap();
        });

        let mut source = GateSubscriptionSource::connect(&sock, SessionId("s1".into())).unwrap();
        let got = [source.next().unwrap(), source.next().unwrap()];

        server.join().unwrap();
        let _ = std::fs::remove_file(&sock);
        assert_eq!(got, [events[0].clone(), events[1].clone()]);
    }

    #[test]
    fn gate_subscription_source_rejects_wire_version_mismatch() {
        let sock = temp_sock("badver");
        let listener = UnixListener::bind(&sock).unwrap();

        let server = thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = conn.read(&mut buf).unwrap();
            conn.write_all(&ready_frame(WIRE_VERSION + 1)).unwrap();
            conn.flush().unwrap();
        });

        let err = GateSubscriptionSource::connect(&sock, SessionId("s1".into())).unwrap_err();

        server.join().unwrap();
        let _ = std::fs::remove_file(&sock);
        assert!(
            err.to_string().contains("wire version"),
            "rejected on version mismatch: {err}"
        );
    }

    #[test]
    fn gate_subscription_source_closes_socket_on_drop() {
        let sock = temp_sock("drop");
        let listener = UnixListener::bind(&sock).unwrap();
        let (tx, rx) = mpsc::channel();

        let server = thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = conn.read(&mut buf).unwrap();
            conn.write_all(&ready_frame(WIRE_VERSION)).unwrap();
            conn.flush().unwrap();
            // Blocks until the client closes its end; read returns 0 at EOF.
            let mut sink = [0u8; 64];
            let n = conn.read(&mut sink).unwrap();
            tx.send(n).unwrap();
        });

        let source = GateSubscriptionSource::connect(&sock, SessionId("s1".into())).unwrap();
        drop(source);

        let observed = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        server.join().unwrap();
        let _ = std::fs::remove_file(&sock);
        assert_eq!(observed, 0, "the gate observes EOF after the source drops");
    }

    #[test]
    fn stub_and_gate_sources_produce_identical_chunks() {
        let events = vec![
            HookEvent {
                session_id: SessionId("s1".into()),
                correlation_id: CorrelationId("c1".into()),
                ts: 1,
                kind: HookKind::SessionStart {
                    cwd: Some("/proj".into()),
                    client: Some("cli".into()),
                    cli_version: None,
                },
            },
            prompt_event("c2", "capture me"),
            HookEvent {
                session_id: SessionId("s1".into()),
                correlation_id: CorrelationId("c3".into()),
                ts: 3,
                kind: HookKind::PostToolUse {
                    tool_name: "Read".into(),
                    tool_input: json!({ "file_path": "src/x.rs" }),
                    tool_response: "...".into(),
                    turn_index: 1,
                },
            },
        ];

        let dir_a = tempfile::tempdir().unwrap();
        let memorya_a = Arc::new(Mutex::new(Engram::open(dir_a.path().join("a.db")).unwrap()));
        let cap_a = HookCapturer::new(memorya_a.clone());
        let mut stub = StubSource::new(events.clone());
        while let Some(e) = stub.next() {
            cap_a.dispatch(&e).unwrap();
        }
        let chunks_a = memorya_a.lock().unwrap().recent_chunks(100).unwrap();

        let sock = temp_sock("identical");
        let listener = UnixListener::bind(&sock).unwrap();
        let server_events = events.clone();
        let server = thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = conn.read(&mut buf).unwrap();
            conn.write_all(&ready_frame(WIRE_VERSION)).unwrap();
            for e in &server_events {
                conn.write_all(&event_frame(e)).unwrap();
            }
            conn.flush().unwrap();
        });

        let dir_b = tempfile::tempdir().unwrap();
        let memorya_b = Arc::new(Mutex::new(Engram::open(dir_b.path().join("b.db")).unwrap()));
        let cap_b = HookCapturer::new(memorya_b.clone());
        let mut gate = GateSubscriptionSource::connect(&sock, SessionId("s1".into())).unwrap();
        for _ in 0..events.len() {
            let e = gate.next().unwrap();
            cap_b.dispatch(&e).unwrap();
        }
        let chunks_b = memorya_b.lock().unwrap().recent_chunks(100).unwrap();

        server.join().unwrap();
        let _ = std::fs::remove_file(&sock);

        let normalize = |chunks: Vec<(i64, Option<String>, String)>| {
            chunks
                .into_iter()
                .map(|(_, title, content)| (title, content))
                .collect::<Vec<_>>()
        };
        assert_eq!(normalize(chunks_a), normalize(chunks_b));
    }
}
