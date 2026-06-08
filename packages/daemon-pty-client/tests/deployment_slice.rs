//! PTY-only deployment slice (rule 8.2): the daemon PTY client codec encodes
//! and decodes the full handshake in-process, without a running daemon.
//!
//! This proves the PTY-only slice is self-contained and correct at the
//! codec level. The live-daemon variant is `#[ignore]`d and requires
//! `ATHING_DAEMON_SOCK` pointing at a running daemon:
//!   cargo test -p daemon-pty-client --test deployment_slice -- --ignored
//!
//! What the in-process test asserts:
//! - `encode_hello` + `encode_subscribe` produce valid length-prefixed frames.
//! - `FrameDecoder::push` reassembles them and the meta decodes to the
//!   expected wire shapes.
//! - A `decode_session_frame` round-trip on a synthetic hello-ack frame
//!   returns `SessionFrame::HelloAck` with the correct version.
//!
//! Together these guarantee the PTY-only slice can send and receive without
//! a daemon — the transport is the only missing piece.

use contracts::SessionId;
use daemon_pty_client::{
    decode_session_frame, encode_hello, encode_subscribe, FrameDecoder, RawFrame, SessionFrame,
    WIRE_VERSION,
};
use serde_json::{json, Value};

#[test]
fn pty_only_slice_hello_encodes_and_decodes_in_process() {
    let bytes = encode_hello(&["snapshot"]);
    let frames = FrameDecoder::new().push(&bytes);
    assert_eq!(frames.len(), 1, "one hello frame");

    let meta: Value = serde_json::from_slice(&frames[0].meta).expect("hello meta is json");
    assert_eq!(meta["type"], "hello");
    assert_eq!(meta["versions"], json!([WIRE_VERSION]));
    assert_eq!(meta["capabilities"], json!(["snapshot"]));
}

#[test]
fn pty_only_slice_subscribe_encodes_session_id() {
    let session = SessionId("s-42".into());
    let bytes = encode_subscribe(&session);
    let frames = FrameDecoder::new().push(&bytes);
    assert_eq!(frames.len(), 1, "one subscribe frame");

    let meta: Value = serde_json::from_slice(&frames[0].meta).expect("subscribe meta is json");
    assert_eq!(meta["type"], "subscribe");
    assert_eq!(meta["sessionId"], "s-42");
}

#[test]
fn pty_only_slice_hello_ack_decodes_to_typed_frame() {
    let raw = RawFrame {
        meta: serde_json::to_vec(&json!({
            "type": "hello-ack",
            "version": WIRE_VERSION,
            "daemonVersion": "0.1.0-test",
            "capabilities": ["pty"]
        }))
        .expect("meta json"),
        body: None,
    };

    let frame = decode_session_frame(&raw).expect("hello-ack decodes");
    match frame {
        SessionFrame::HelloAck {
            version,
            daemon_version,
            capabilities,
        } => {
            assert_eq!(version, WIRE_VERSION);
            assert_eq!(daemon_version, "0.1.0-test");
            assert!(capabilities.contains(&"pty".to_string()));
        }
        other => panic!("expected HelloAck, got {other:?}"),
    }
}

/// Live PTY-only slice: connect to a running daemon, send hello, assert
/// the ack negotiates the wire version.
///
/// Requires a live daemon:
///   ATHING_DAEMON_SOCK=~/.athing/daemon.sock \
///   cargo test -p daemon-pty-client --test deployment_slice \
///     pty_only_slice_live_daemon -- --ignored
#[test]
#[ignore = "requires a live daemon; set ATHING_DAEMON_SOCK and run with --ignored"]
fn pty_only_slice_live_daemon_negotiates_wire_version() {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    let sock = std::env::var("ATHING_DAEMON_SOCK")
        .expect("ATHING_DAEMON_SOCK points at a running daemon socket");

    let mut stream = UnixStream::connect(&sock).expect("connect to daemon");
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();

    let hello = encode_hello(&["snapshot"]);
    stream.write_all(&hello).expect("write hello");

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).expect("read hello-ack");
    assert!(n > 0, "daemon sent a hello-ack");

    let frames = FrameDecoder::new().push(&buf[..n]);
    assert!(!frames.is_empty(), "at least one frame in the ack");

    let ack = decode_session_frame(&frames[0]).expect("frame decodes");
    match ack {
        SessionFrame::HelloAck { version, .. } => {
            assert_eq!(version, WIRE_VERSION, "negotiated version matches client");
        }
        other => panic!("expected HelloAck from daemon, got {other:?}"),
    }
}
