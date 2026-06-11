//! PTY session-event wire codec (sole Rust owner of ADR-0009 framing).
//! Consumer provides transport; this crate provides encode/decode only.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use contracts::{SessionId, SESSION_EVENT_WIRE_VERSION};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const HEADER_SIZE: usize = 4;
const BODY_SEP: u8 = 0x0a;

/// The PTY session-event wire version this client speaks (R9: sourced from
/// `contracts`, negotiated with the daemon at `hello`).
pub const WIRE_VERSION: u32 = SESSION_EVENT_WIRE_VERSION;

/// A frame split into its JSON meta and optional raw body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFrame {
    /// The JSON meta bytes (never contains a raw `0x0a`).
    pub meta: Vec<u8>,
    /// The raw body bytes, when the frame carried one.
    pub body: Option<Vec<u8>>,
}

/// Encode a frame: 4-byte big-endian payload length, JSON meta, then an optional
/// `0x0a` separator and raw body. Mirrors the daemon's `encode_frame`.
pub fn encode_frame(meta: &[u8], body: Option<&[u8]>) -> Vec<u8> {
    let payload_len = match body {
        Some(b) => meta.len() + 1 + b.len(),
        None => meta.len(),
    };
    let mut out = Vec::with_capacity(HEADER_SIZE + payload_len);
    out.extend_from_slice(&(payload_len as u32).to_be_bytes());
    out.extend_from_slice(meta);
    if let Some(b) = body {
        out.push(BODY_SEP);
        out.extend_from_slice(b);
    }
    out
}

/// Incremental decoder: feed it socket chunks, get back complete frames. Holds a
/// partial frame across pushes.
#[derive(Default)]
pub struct FrameDecoder {
    buf: Vec<u8>,
}

impl FrameDecoder {
    /// A fresh decoder with an empty buffer.
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Append `chunk` and return every complete frame now available.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<RawFrame> {
        self.buf.extend_from_slice(chunk);
        let mut results = Vec::new();
        let mut offset = 0usize;

        while self.buf.len() - offset >= HEADER_SIZE {
            let len_bytes = [
                self.buf[offset],
                self.buf[offset + 1],
                self.buf[offset + 2],
                self.buf[offset + 3],
            ];
            let payload_len = u32::from_be_bytes(len_bytes) as usize;
            if self.buf.len() - offset < HEADER_SIZE + payload_len {
                break;
            }
            let start = offset + HEADER_SIZE;
            let end = start + payload_len;
            let payload = &self.buf[start..end];

            // JSON.stringify never emits a raw newline, so the first 0x0a is the
            // meta/body separator.
            let frame = match payload.iter().position(|&b| b == BODY_SEP) {
                Some(nl) => RawFrame {
                    meta: payload[..nl].to_vec(),
                    body: Some(payload[nl + 1..].to_vec()),
                },
                None => RawFrame {
                    meta: payload.to_vec(),
                    body: None,
                },
            };
            results.push(frame);
            offset = end;
        }

        if offset > 0 {
            self.buf.drain(..offset);
        }
        results
    }
}

/// Encode the `hello` handshake a consumer sends: the wire version it speaks and
/// the capabilities it requests (the daemon is pty-only, so `["snapshot"]`).
pub fn encode_hello(capabilities: &[&str]) -> Vec<u8> {
    let meta = json!({
        "type": "hello",
        "versions": [WIRE_VERSION],
        "capabilities": capabilities,
    });
    encode_frame(&serde_json::to_vec(&meta).expect("hello meta"), None)
}

/// Encode a `subscribe` request for a session's event stream.
pub fn encode_subscribe(session: &SessionId) -> Vec<u8> {
    let meta = json!({ "type": "subscribe", "sessionId": session.0 });
    encode_frame(&serde_json::to_vec(&meta).expect("subscribe meta"), None)
}

/// Parameters for a `spawn` frame. `command: None` lets the daemon resolve the
/// user's login shell; `resume` carries a prior session id to revive.
#[derive(Debug, Clone)]
pub struct SpawnParams<'a> {
    /// The client-chosen session id (the orchestrator passes the `surface_id`).
    pub session_id: &'a SessionId,
    /// Per-session token recorded by the daemon.
    pub token: &'a str,
    /// Initial terminal width in columns.
    pub cols: u16,
    /// Initial terminal height in rows.
    pub rows: u16,
    /// Working directory for the spawned process.
    pub cwd: &'a str,
    /// Command to launch; `None` resolves the login shell.
    pub command: Option<&'a str>,
    /// Arguments passed to `command`.
    pub args: &'a [String],
    /// Extra environment entries; `None` inherits the daemon default.
    pub env: Option<&'a BTreeMap<String, String>>,
    /// A prior session id to resume, when reviving a stopped session.
    pub resume: Option<&'a str>,
}

/// Encode a `spawn` frame that creates (or resumes) a PTY session under a
/// client-chosen session id.
pub fn encode_spawn(params: &SpawnParams<'_>) -> Vec<u8> {
    let mut meta = json!({
        "type": "spawn",
        "sessionId": params.session_id.0,
        "token": params.token,
        "cols": params.cols,
        "rows": params.rows,
        "cwd": params.cwd,
        "args": params.args,
    });
    let obj = meta.as_object_mut().expect("spawn meta object");
    if let Some(command) = params.command {
        obj.insert("command".into(), json!(command));
    }
    if let Some(env) = params.env {
        obj.insert("env".into(), json!(env));
    }
    if let Some(resume) = params.resume {
        obj.insert("resume".into(), json!(resume));
    }
    encode_frame(&serde_json::to_vec(&meta).expect("spawn meta"), None)
}

/// Encode an `input` frame carrying raw bytes for a session's PTY stdin.
pub fn encode_input(session: &SessionId, bytes: &[u8]) -> Vec<u8> {
    let meta = json!({ "type": "input", "sessionId": session.0 });
    encode_frame(&serde_json::to_vec(&meta).expect("input meta"), Some(bytes))
}

/// Encode a `resize` frame setting a session's terminal dimensions.
pub fn encode_resize(session: &SessionId, cols: u16, rows: u16) -> Vec<u8> {
    let meta = json!({ "type": "resize", "sessionId": session.0, "cols": cols, "rows": rows });
    encode_frame(&serde_json::to_vec(&meta).expect("resize meta"), None)
}

/// Encode an `ack` frame returning `bytes` of flow-control credit for a session.
pub fn encode_ack(session: &SessionId, bytes: i64) -> Vec<u8> {
    let meta = json!({ "type": "ack", "sessionId": session.0, "bytes": bytes });
    encode_frame(&serde_json::to_vec(&meta).expect("ack meta"), None)
}

/// Encode a `kill` frame requesting graceful termination of a session.
pub fn encode_kill(session: &SessionId) -> Vec<u8> {
    let meta = json!({ "type": "kill", "sessionId": session.0 });
    encode_frame(&serde_json::to_vec(&meta).expect("kill meta"), None)
}

/// Encode a `stop` frame: terminate a session and suppress auto-resume.
pub fn encode_stop(session: &SessionId) -> Vec<u8> {
    let meta = json!({ "type": "stop", "sessionId": session.0 });
    encode_frame(&serde_json::to_vec(&meta).expect("stop meta"), None)
}

/// Encode an `unsubscribe` frame detaching from a session's event stream while
/// leaving the session running.
pub fn encode_unsubscribe(session: &SessionId) -> Vec<u8> {
    let meta = json!({ "type": "unsubscribe", "sessionId": session.0 });
    encode_frame(&serde_json::to_vec(&meta).expect("unsubscribe meta"), None)
}

/// A decoded session-event frame: a handshake ack, raw output bytes, or a
/// lifecycle event by session id. Consumer-oblivious — no hook plane.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionFrame {
    /// The daemon's handshake reply.
    HelloAck {
        /// Negotiated wire version.
        version: u32,
        /// The daemon's build version.
        daemon_version: String,
        /// Capabilities the daemon advertises (pty-only: no hook).
        capabilities: Vec<String>,
    },
    /// The daemon's acknowledgement that a `spawn` started a session.
    SpawnAck {
        /// The session the daemon started (echoes the requested id).
        session_id: String,
        /// The spawned process id.
        pid: u32,
    },
    /// Raw PTY output for a session.
    Data {
        /// Session the bytes belong to.
        session_id: String,
        /// The raw output bytes.
        bytes: Vec<u8>,
    },
    /// A terminal-derived status transition.
    Status {
        /// Session id.
        session_id: String,
        /// The new status (e.g. `IDLE`, `WORKING`).
        status: String,
        /// The plane the status was derived from (e.g. `terminal`).
        source: String,
    },
    /// A session exited.
    Exit {
        /// Session id.
        session_id: String,
        /// The qualified exit reason.
        qualifier: String,
    },
    /// The daemon rejected a request or reported a session-level fault.
    Error {
        /// The session the error concerns, when scoped to one.
        session_id: Option<String>,
        /// The daemon's error code (e.g. `ENOENT`, `EEXIST`).
        code: String,
        /// A human-readable message.
        message: String,
    },
    /// A frame whose `type` this client does not model (forward-compatible).
    Other {
        /// The unrecognized frame `type`.
        kind: String,
    },
}

/// Decode a [`RawFrame`] into a typed [`SessionFrame`]. Returns `None` only when
/// the meta is not valid JSON or carries no string `type`.
pub fn decode_session_frame(frame: &RawFrame) -> Option<SessionFrame> {
    let meta: Value = serde_json::from_slice(&frame.meta).ok()?;
    let kind = meta.get("type")?.as_str()?;
    let sid = |m: &Value| {
        m.get("sessionId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };

    Some(match kind {
        "hello-ack" => SessionFrame::HelloAck {
            version: meta.get("version").and_then(Value::as_u64).unwrap_or(0) as u32,
            daemon_version: meta
                .get("daemonVersion")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            capabilities: meta
                .get("capabilities")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
        },
        "spawn-ack" => SessionFrame::SpawnAck {
            session_id: sid(&meta),
            pid: meta.get("pid").and_then(Value::as_u64).unwrap_or(0) as u32,
        },
        "data" => SessionFrame::Data {
            session_id: sid(&meta),
            bytes: frame.body.clone().unwrap_or_default(),
        },
        "status" => SessionFrame::Status {
            session_id: sid(&meta),
            status: meta
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            source: meta
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        },
        "exit" => SessionFrame::Exit {
            session_id: sid(&meta),
            qualifier: meta
                .get("qualifier")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        },
        "error" => SessionFrame::Error {
            session_id: meta
                .get("sessionId")
                .and_then(Value::as_str)
                .map(str::to_string),
            code: meta
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            message: meta
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        },
        other => SessionFrame::Other {
            kind: other.to_string(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_advertises_the_contracts_wire_version() {
        let bytes = encode_hello(&["snapshot"]);
        let frame = &FrameDecoder::new().push(&bytes)[0];
        let meta: Value = serde_json::from_slice(&frame.meta).unwrap();
        assert_eq!(meta["versions"], json!([SESSION_EVENT_WIRE_VERSION]));
        assert_eq!(meta["type"], "hello");
        assert_eq!(meta["capabilities"], json!(["snapshot"]));
    }

    #[test]
    fn subscribe_carries_the_session_id() {
        let bytes = encode_subscribe(&SessionId("s1".into()));
        let frame = &FrameDecoder::new().push(&bytes)[0];
        let meta: Value = serde_json::from_slice(&frame.meta).unwrap();
        assert_eq!(meta, json!({ "type": "subscribe", "sessionId": "s1" }));
    }

    #[test]
    fn spawn_carries_session_token_and_geometry() {
        let bytes = encode_spawn(&SpawnParams {
            session_id: &SessionId("surf1".into()),
            token: "tok",
            cols: 80,
            rows: 24,
            cwd: "/tmp",
            command: None,
            args: &[],
            env: None,
            resume: None,
        });
        let frame = &FrameDecoder::new().push(&bytes)[0];
        let meta: Value = serde_json::from_slice(&frame.meta).unwrap();
        assert_eq!(
            meta,
            json!({
                "type": "spawn", "sessionId": "surf1", "token": "tok",
                "cols": 80, "rows": 24, "cwd": "/tmp", "args": []
            })
        );
    }

    #[test]
    fn spawn_includes_optional_command_and_resume() {
        let args = vec!["--flag".to_string()];
        let bytes = encode_spawn(&SpawnParams {
            session_id: &SessionId("s".into()),
            token: "t",
            cols: 10,
            rows: 10,
            cwd: "/",
            command: Some("/bin/sh"),
            args: &args,
            env: None,
            resume: Some("prior"),
        });
        let frame = &FrameDecoder::new().push(&bytes)[0];
        let meta: Value = serde_json::from_slice(&frame.meta).unwrap();
        assert_eq!(
            meta,
            json!({
                "type": "spawn", "sessionId": "s", "token": "t",
                "cols": 10, "rows": 10, "cwd": "/", "args": ["--flag"],
                "command": "/bin/sh", "resume": "prior"
            })
        );
    }

    #[test]
    fn input_carries_raw_body_bytes() {
        let bytes = encode_input(&SessionId("s1".into()), &[0x0a, 0x1b, 0x41]);
        let frames = FrameDecoder::new().push(&bytes);
        let meta: Value = serde_json::from_slice(&frames[0].meta).unwrap();
        assert_eq!(meta, json!({ "type": "input", "sessionId": "s1" }));
        assert_eq!(frames[0].body.as_deref(), Some(&[0x0a, 0x1b, 0x41][..]));
    }

    #[test]
    fn resize_carries_geometry() {
        let bytes = encode_resize(&SessionId("s1".into()), 120, 40);
        let frame = &FrameDecoder::new().push(&bytes)[0];
        let meta: Value = serde_json::from_slice(&frame.meta).unwrap();
        assert_eq!(
            meta,
            json!({ "type": "resize", "sessionId": "s1", "cols": 120, "rows": 40 })
        );
    }

    #[test]
    fn ack_returns_credit() {
        let bytes = encode_ack(&SessionId("s1".into()), 256);
        let frame = &FrameDecoder::new().push(&bytes)[0];
        let meta: Value = serde_json::from_slice(&frame.meta).unwrap();
        assert_eq!(
            meta,
            json!({ "type": "ack", "sessionId": "s1", "bytes": 256 })
        );
    }

    #[test]
    fn kill_stop_unsubscribe_carry_the_session_id() {
        for (bytes, ty) in [
            (encode_kill(&SessionId("s".into())), "kill"),
            (encode_stop(&SessionId("s".into())), "stop"),
            (encode_unsubscribe(&SessionId("s".into())), "unsubscribe"),
        ] {
            let frame = &FrameDecoder::new().push(&bytes)[0];
            let meta: Value = serde_json::from_slice(&frame.meta).unwrap();
            assert_eq!(meta, json!({ "type": ty, "sessionId": "s" }));
        }
    }

    #[test]
    fn decodes_a_spawn_ack_frame() {
        let raw = RawFrame {
            meta: br#"{"type":"spawn-ack","sessionId":"s1","pid":4242}"#.to_vec(),
            body: None,
        };
        assert_eq!(
            decode_session_frame(&raw),
            Some(SessionFrame::SpawnAck {
                session_id: "s1".into(),
                pid: 4242,
            })
        );
    }

    #[test]
    fn decodes_a_data_frame_body_as_raw_bytes() {
        let raw = RawFrame {
            meta: br#"{"type":"data","sessionId":"s1","bodyLen":3}"#.to_vec(),
            body: Some(vec![0x0a, 0x0a, 0x41]),
        };
        assert_eq!(
            decode_session_frame(&raw),
            Some(SessionFrame::Data {
                session_id: "s1".into(),
                bytes: vec![0x0a, 0x0a, 0x41],
            })
        );
    }

    #[test]
    fn decodes_a_status_frame() {
        let raw = RawFrame {
            meta: br#"{"type":"status","sessionId":"s1","status":"IDLE","source":"terminal"}"#
                .to_vec(),
            body: None,
        };
        assert_eq!(
            decode_session_frame(&raw),
            Some(SessionFrame::Status {
                session_id: "s1".into(),
                status: "IDLE".into(),
                source: "terminal".into(),
            })
        );
    }

    #[test]
    fn decodes_an_exit_frame() {
        let raw = RawFrame {
            meta: br#"{"type":"exit","sessionId":"s1","qualifier":"ok"}"#.to_vec(),
            body: None,
        };
        assert_eq!(
            decode_session_frame(&raw),
            Some(SessionFrame::Exit {
                session_id: "s1".into(),
                qualifier: "ok".into(),
            })
        );
    }

    #[test]
    fn decodes_an_error_frame() {
        let raw = RawFrame {
            meta:
                br#"{"type":"error","code":"ENOENT","message":"no such session","sessionId":"s1"}"#
                    .to_vec(),
            body: None,
        };
        assert_eq!(
            decode_session_frame(&raw),
            Some(SessionFrame::Error {
                session_id: Some("s1".into()),
                code: "ENOENT".into(),
                message: "no such session".into(),
            })
        );
    }

    #[test]
    fn unknown_frame_type_is_forward_compatible() {
        let raw = RawFrame {
            meta: br#"{"type":"future-thing"}"#.to_vec(),
            body: None,
        };
        assert_eq!(
            decode_session_frame(&raw),
            Some(SessionFrame::Other {
                kind: "future-thing".into(),
            })
        );
    }

    #[test]
    fn frame_round_trips_through_encode_and_decode() {
        let body = [1u8, 2, 3];
        let bytes = encode_frame(br#"{"type":"data","sessionId":"s1"}"#, Some(&body));
        let frames = FrameDecoder::new().push(&bytes);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].body.as_deref(), Some(&body[..]));
    }

    #[test]
    fn partial_frame_is_held_across_pushes() {
        let bytes = encode_hello(&["snapshot"]);
        let mid = bytes.len() / 2;
        let mut dec = FrameDecoder::new();
        assert_eq!(dec.push(&bytes[..mid]).len(), 0);
        assert_eq!(dec.push(&bytes[mid..]).len(), 1);
    }
}
