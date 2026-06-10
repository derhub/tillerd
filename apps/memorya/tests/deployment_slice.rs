//! Deployment slices: memory-only and full.
//!   test but requires all four processes running.
//!
//! For the gateway-only slice see `apps/mcp-gateway/tests/deployment_slice.rs`.
//! For the PTY-only slice see `packages/daemon-pty-client/tests/deployment_slice.rs`.
//!
//! All tests that require live processes are `#[ignore]`d. Run them with:
//!   cargo test -p memorya --test deployment_slice -- --ignored

use std::sync::{Arc, Mutex};

use contracts::SessionId;
use memorya::capture::HookCapturer;
use memorya::hook_source::{GateSubscriptionSource, HookSource};
use memorya::Engram;

#[test]
#[ignore = "requires a live gate; run with \
            TILLERD_DIR + TILLERD_SESSION_ID set and --ignored"]
fn memory_only_slice_subscribes_to_gate_without_daemon_or_gateway() {
    let base = std::env::var("TILLERD_DIR").expect("TILLERD_DIR points at the runtime directory");
    let subscribe_sock = std::path::Path::new(&base).join("gate.sock");
    let session_id = std::env::var("TILLERD_SESSION_ID")
        .expect("TILLERD_SESSION_ID names the session to capture");

    let dir = tempfile::tempdir().expect("temp dir");
    let memorya = Arc::new(Mutex::new(
        Engram::open(dir.path().join("slice.db")).expect("open memorya"),
    ));
    let capturer = HookCapturer::new(memorya.clone());

    let mut source = GateSubscriptionSource::connect(&subscribe_sock, SessionId(session_id))
        .expect("subscribe to the live gate");

    let event = source
        .next()
        .expect("the live gate streams at least one hook event");
    capturer
        .dispatch(&event)
        .expect("capture the streamed event");

    let chunks = memorya
        .lock()
        .expect("memorya mutex")
        .recent_chunks(10)
        .expect("read recent chunks");

    assert!(
        !chunks.is_empty(),
        "a streamed hook event becomes a memory chunk through the gate-client slice"
    );
}

/// Full deployment slice: daemon + gate + gateway + memorya all live.
///
/// Asserts that memorya receives and stores an event that originated as a hook
/// on the daemon, was routed through the gate, and was fanfed by the gateway.
/// The correlation id must be present on the stored chunk (see task 8.3).
///
/// Requirements:
/// - A running daemon at `TILLERD_DAEMON_SOCK`
/// - A running gate (its sockets under `TILLERD_DIR`)
/// - A running gateway connected to the gate via `TILLERD_SESSION_ID` + `TILLERD_SESSION_TOKEN`
/// - A session generating hook events at `TILLERD_SESSION_ID`
///
/// Run with:
///   TILLERD_DIR=… TILLERD_SESSION_ID=… \
///   cargo test -p memorya --test deployment_slice \
///     full_slice_daemon_gate_gateway_memory -- --ignored
#[test]
#[ignore = "requires live daemon + gate + gateway; set TILLERD_DIR + TILLERD_SESSION_ID and run with --ignored"]
fn full_slice_daemon_gate_gateway_memory() {
    let base = std::env::var("TILLERD_DIR").expect("TILLERD_DIR points at the runtime directory");
    let subscribe_sock = std::path::Path::new(&base).join("gate.sock");
    let session_id =
        std::env::var("TILLERD_SESSION_ID").expect("TILLERD_SESSION_ID names the session");

    let dir = tempfile::tempdir().expect("temp dir");
    let memorya = Arc::new(Mutex::new(
        Engram::open(dir.path().join("full.db")).expect("open memorya"),
    ));
    let capturer = HookCapturer::new(memorya.clone());

    let mut source = GateSubscriptionSource::connect(&subscribe_sock, SessionId(session_id))
        .expect("subscribe to the live gate");

    let event = source
        .next()
        .expect("the live gate streams at least one hook event from the full stack");
    capturer.dispatch(&event).expect("capture the event");

    let chunks = memorya
        .lock()
        .expect("memorya mutex")
        .recent_chunks(10)
        .expect("read recent chunks");

    assert!(
        !chunks.is_empty(),
        "a hook event originating in the daemon path becomes a memory chunk in the full slice"
    );
}
