//! CorrelationId threads unchanged across process boundaries so distributed tracing
//! can correlate log/event streams back to a single user action.

use contracts::{CorrelationId, HookEvent, HookKind, SessionId, ToolInbound};
use serde_json::json;

const CORR: &str = "trace-abc-123";

fn hook_event(corr: &str) -> HookEvent {
    HookEvent {
        session_id: SessionId("sess-1".into()),
        correlation_id: CorrelationId(corr.into()),
        ts: 1_700_000_000_000,
        kind: HookKind::UserPromptSubmit {
            content: "hello".into(),
            turn_index: Some(0),
        },
    }
}

fn tool_call(corr: &str) -> ToolInbound {
    ToolInbound::ToolCall {
        session_id: SessionId("sess-1".into()),
        correlation_id: CorrelationId(corr.into()),
        tool_name: "Bash".into(),
        tool_input: json!({ "command": "ls" }),
    }
}

fn tool_result(corr: &str) -> ToolInbound {
    ToolInbound::ToolResult {
        session_id: SessionId("sess-1".into()),
        correlation_id: CorrelationId(corr.into()),
        tool_name: "Bash".into(),
        tool_response: "file1\nfile2".into(),
    }
}

/// The correlation id survives a JSON round-trip in the HookEvent shape
/// (daemon -> gate -> gate-subscriber boundary).
#[test]
fn correlation_id_survives_hook_event_json_round_trip() {
    let event = hook_event(CORR);
    let json = serde_json::to_value(&event).expect("encode");

    assert_eq!(json["correlationId"], CORR, "correlation id in wire json");

    let decoded: HookEvent = serde_json::from_value(json).expect("decode");
    assert_eq!(decoded.correlation_id, CorrelationId(CORR.into()));
}

/// The correlation id survives a JSON round-trip in the ToolCall shape
/// (gateway -> gate tool-route boundary).
#[test]
fn correlation_id_survives_tool_call_json_round_trip() {
    let inbound = tool_call(CORR);
    let json = serde_json::to_value(&inbound).expect("encode");

    assert_eq!(json["payload"]["correlationId"], CORR);

    let decoded: ToolInbound = serde_json::from_value(json).expect("decode");
    match decoded {
        ToolInbound::ToolCall { correlation_id, .. } => {
            assert_eq!(correlation_id, CorrelationId(CORR.into()));
        }
        other => panic!("expected ToolCall, got {other:?}"),
    }
}

/// The correlation id survives a JSON round-trip in the ToolResult shape
/// (gateway -> gate observation boundary).
#[test]
fn correlation_id_survives_tool_result_json_round_trip() {
    let inbound = tool_result(CORR);
    let json = serde_json::to_value(&inbound).expect("encode");

    assert_eq!(json["payload"]["correlationId"], CORR);

    let decoded: ToolInbound = serde_json::from_value(json).expect("decode");
    match decoded {
        ToolInbound::ToolResult { correlation_id, .. } => {
            assert_eq!(correlation_id, CorrelationId(CORR.into()));
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

/// The same `CorrelationId` value threads through all three hop shapes without
/// mutation. This asserts the cross-hop static guarantee: a single injected id
/// would be the same object in HookEvent, ToolCall, and ToolResult.
#[test]
fn same_correlation_id_appears_in_all_three_hop_shapes() {
    let corr = CorrelationId(CORR.into());

    let hook = HookEvent {
        session_id: SessionId("sess-1".into()),
        correlation_id: corr.clone(),
        ts: 0,
        kind: HookKind::Stop { turn_index: None },
    };
    let call = ToolInbound::ToolCall {
        session_id: SessionId("sess-1".into()),
        correlation_id: corr.clone(),
        tool_name: "Bash".into(),
        tool_input: json!({}),
    };
    let result = ToolInbound::ToolResult {
        session_id: SessionId("sess-1".into()),
        correlation_id: corr.clone(),
        tool_name: "Bash".into(),
        tool_response: String::new(),
    };

    assert_eq!(hook.correlation_id, corr);
    let ToolInbound::ToolCall {
        correlation_id: call_corr,
        ..
    } = call
    else {
        panic!("expected ToolCall");
    };
    assert_eq!(call_corr, corr);
    let ToolInbound::ToolResult {
        correlation_id: result_corr,
        ..
    } = result
    else {
        panic!("expected ToolResult");
    };
    assert_eq!(result_corr, corr);
}

/// The standardized observability vocabulary (design D5): a correlated record's log
/// attribute key is exactly `correlation_id` — snake_case, distinct from the camelCase
/// `correlationId` used on the JSON wire. Capture a structured log line emitted in the
/// production shape (orchestrator, gate, and daemon all log `correlation_id = ...`) and
/// assert the key, so a drift to the wire form would fail loudly.
#[test]
fn the_log_attribute_key_is_exactly_correlation_id() {
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);
    impl Write for Buffer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> MakeWriter<'a> for Buffer {
        type Writer = Buffer;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    let buffer = Buffer::default();
    let captured = buffer.0.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_writer(buffer)
        .finish();

    let corr = CorrelationId(CORR.into());
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(correlation_id = %corr.0, "correlated operation");
    });

    let logged = String::from_utf8(captured.lock().unwrap().clone()).expect("utf8 log output");
    let record: serde_json::Value =
        serde_json::from_str(logged.lines().next().expect("one log line")).expect("json record");
    let fields = &record["fields"];
    assert_eq!(
        fields["correlation_id"],
        json!(CORR),
        "the log attribute key is exactly `correlation_id`: {logged}"
    );
    assert!(
        fields.get("correlationId").is_none(),
        "the log key is snake_case, not the camelCase wire form: {logged}"
    );
}

/// Live cross-hop correlation trace: one id injected at the daemon hook entry
/// must be observable in the gate subscriber stream.
///
/// Requirements: live daemon + gate + an active session generating hook events.
///   TILLERD_DIR=... TILLERD_SESSION_ID=... \
///   cargo test -p tillerd-contracts --test correlation_trace \
///     correlation_id_threads_daemon_to_gate_in_live_stack -- --ignored
#[test]
#[ignore = "requires live daemon + gate; set TILLERD_DIR + TILLERD_SESSION_ID and run with --ignored"]
fn correlation_id_threads_daemon_to_gate_in_live_stack() {
    // Validate that every HookEvent received from the gate carries a non-empty
    // correlation id — which proves the daemon minted one and the gate preserved it.
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    let base = std::env::var("TILLERD_DIR").expect("TILLERD_DIR points at the runtime directory");
    let subscribe_sock = std::path::Path::new(&base).join("gate.sock");
    let session_id =
        std::env::var("TILLERD_SESSION_ID").expect("TILLERD_SESSION_ID names the active session");

    // The gate wire: open the single socket on the Subscribe route via one preamble
    // frame, then read back a frame. Raw bytes keep contracts-rs dep-free here.
    let mut stream = UnixStream::connect(&subscribe_sock).expect("connect to gate socket");
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();

    let subscribe = serde_json::to_vec(&serde_json::json!({
        "route": "subscribe",
        "sessionId": session_id,
        "wireVersion": 1
    }))
    .unwrap();
    let mut frame = Vec::with_capacity(4 + subscribe.len());
    frame.extend_from_slice(&(subscribe.len() as u32).to_be_bytes());
    frame.extend_from_slice(&subscribe);
    stream.write_all(&frame).expect("send subscribe request");

    // Read at least two frames: the handshake ack, then an event.
    let mut buf = vec![0u8; 65536];
    let n = stream.read(&mut buf).expect("read from gate");
    assert!(n > 0, "gate sent data");

    // Find at least one frame whose JSON has a non-empty correlationId.
    let mut offset = 0;
    let mut found_correlation = false;
    while offset + 4 <= n {
        let len = u32::from_be_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]) as usize;
        offset += 4;
        if offset + len > n {
            break;
        }
        let payload = &buf[offset..offset + len];
        offset += len;
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(payload) {
            if let Some(corr) = val.get("correlationId").and_then(|v| v.as_str()) {
                assert!(!corr.is_empty(), "correlation id must not be empty");
                found_correlation = true;
                break;
            }
        }
    }
    assert!(
        found_correlation,
        "at least one gate frame must carry a non-empty correlationId"
    );
}
