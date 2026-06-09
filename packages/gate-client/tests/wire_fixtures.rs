//! Golden wire tests: decoder must round-trip real hook-subscription codec payloads.

use contracts::{
    CorrelationId, HookEvent, HookKind, Route, RoutePreamble, SessionId,
    HOOK_SUBSCRIPTION_WIRE_VERSION,
};
use gate_client::{
    decode_subscription_frame, encode_frame, encode_subscribe_preamble, negotiate_ready,
    DecodeError, FrameDecoder, RawFrame, SubscriptionFrame, WIRE_VERSION,
};
use serde_json::{json, Value};

fn raw(payload: &str) -> RawFrame {
    RawFrame {
        payload: payload.trim().as_bytes().to_vec(),
    }
}

fn event(correlation: &str, kind: HookKind) -> HookEvent {
    HookEvent {
        session_id: SessionId("sess-7f3a".into()),
        correlation_id: CorrelationId(correlation.into()),
        ts: 1_717_000_000_000,
        kind,
    }
}

fn all_hook_kinds() -> Vec<HookKind> {
    vec![
        HookKind::SessionStart {
            cwd: Some("/home/dev/proj".into()),
            client: Some("agent-cli".into()),
            cli_version: Some("1.4.2".into()),
        },
        HookKind::UserPromptSubmit {
            content: "summarize the auth module".into(),
            turn_index: Some(0),
        },
        HookKind::PostToolUse {
            tool_name: "Read".into(),
            tool_input: json!({ "file_path": "src/auth.rs" }),
            tool_response: "pub fn login() {}".into(),
            turn_index: 1,
        },
        HookKind::PermissionRequest {
            tool_name: Some("Bash".into()),
            request: json!({ "command": "rm -rf /tmp/x" }),
        },
        HookKind::Stop {
            turn_index: Some(2),
        },
        HookKind::SessionEnd {
            reason: Some("user quit".into()),
        },
    ]
}

#[test]
fn frame_decoder_holds_partial_frame_across_pushes() {
    let bytes = encode_frame(br#"{"frame":"ready","wireVersion":1}"#);
    let mid = bytes.len() / 2;
    let mut decoder = FrameDecoder::new();

    assert_eq!(decoder.push(&bytes[..mid]).unwrap().len(), 0);
    assert_eq!(decoder.push(&bytes[mid..]).unwrap().len(), 1);
}

#[test]
fn frame_decoder_extracts_multiple_frames_in_one_push() {
    let mut stream = encode_frame(b"first");
    stream.extend_from_slice(&encode_frame(b"second"));

    let frames = FrameDecoder::new().push(&stream).unwrap();

    assert_eq!(
        frames,
        vec![
            RawFrame {
                payload: b"first".to_vec()
            },
            RawFrame {
                payload: b"second".to_vec()
            },
        ]
    );
}

#[test]
fn encode_frame_round_trips_through_frame_decoder() {
    let frames = FrameDecoder::new().push(&encode_frame(b"payload")).unwrap();

    assert_eq!(frames[0].payload, b"payload");
}

#[test]
fn encode_subscribe_preamble_carries_route_session_id_and_wire_version() {
    let bytes = encode_subscribe_preamble(&SessionId("s1".into()));
    let frames = FrameDecoder::new().push(&bytes).unwrap();
    let meta: Value = serde_json::from_slice(&frames[0].payload).unwrap();

    assert_eq!(
        meta,
        json!({ "route": "subscribe", "sessionId": "s1", "wireVersion": WIRE_VERSION })
    );
}

#[test]
fn encode_subscribe_preamble_round_trips_through_frame_decoder() {
    let frames = FrameDecoder::new()
        .push(&encode_subscribe_preamble(&SessionId("s1".into())))
        .unwrap();
    let decoded: RoutePreamble = serde_json::from_slice(&frames[0].payload).unwrap();

    assert_eq!(
        decoded,
        RoutePreamble {
            route: Route::Subscribe,
            session_id: Some(SessionId("s1".into())),
            token: None,
            wire_version: WIRE_VERSION,
        }
    );
}

#[test]
fn decode_subscription_frame_parses_ready_wire_version() {
    assert_eq!(
        decode_subscription_frame(&raw(r#"{"frame":"ready","wireVersion":1}"#)),
        Some(SubscriptionFrame::Ready { wire_version: 1 })
    );
}

#[test]
fn decode_subscription_frame_parses_hook_event_for_each_hook_kind() {
    for (i, kind) in all_hook_kinds().into_iter().enumerate() {
        let original = event(&format!("corr-{i}"), kind);
        let payload = serde_json::to_vec(&json!({ "frame": "event", "event": original })).unwrap();

        assert_eq!(
            decode_subscription_frame(&RawFrame { payload }),
            Some(SubscriptionFrame::Event(original))
        );
    }
}

#[test]
fn decode_subscription_frame_preserves_correlation_id_unchanged() {
    let original = event(
        "corr-keep-me",
        HookKind::Stop {
            turn_index: Some(9),
        },
    );
    let payload = serde_json::to_vec(&json!({ "frame": "event", "event": original })).unwrap();

    let decoded = decode_subscription_frame(&RawFrame { payload }).unwrap();

    match decoded {
        SubscriptionFrame::Event(e) => {
            assert_eq!(e.correlation_id, CorrelationId("corr-keep-me".into()))
        }
        other => panic!("expected an event frame, got {other:?}"),
    }
}

#[test]
fn decode_subscription_frame_parses_error_reason() {
    assert_eq!(
        decode_subscription_frame(&raw(
            r#"{"frame":"error","reason":"unsupported wire version"}"#
        )),
        Some(SubscriptionFrame::Error {
            reason: "unsupported wire version".into()
        })
    );
}

#[test]
fn decode_subscription_frame_returns_other_on_unknown_frame() {
    assert_eq!(
        decode_subscription_frame(&raw(r#"{"frame":"future-thing"}"#)),
        Some(SubscriptionFrame::Other {
            frame: "future-thing".into()
        })
    );
}

#[test]
fn decode_subscription_frame_returns_none_on_invalid_json() {
    assert_eq!(decode_subscription_frame(&raw("not json")), None);
}

#[test]
fn wire_version_equals_contracts_hook_subscription_wire_version() {
    assert_eq!(WIRE_VERSION, HOOK_SUBSCRIPTION_WIRE_VERSION);
}

#[test]
fn subscribe_decode_matches_hook_subscription_wire_fixtures() {
    let cases = [
        (
            include_str!("fixtures/ready.json"),
            SubscriptionFrame::Ready {
                wire_version: WIRE_VERSION,
            },
        ),
        (
            include_str!("fixtures/error.json"),
            SubscriptionFrame::Error {
                reason: "unsupported wire version".into(),
            },
        ),
        (
            include_str!("fixtures/session_start.json"),
            SubscriptionFrame::Event(event("corr-1", all_hook_kinds().remove(0))),
        ),
        (
            include_str!("fixtures/user_prompt_submit.json"),
            SubscriptionFrame::Event(HookEvent {
                ts: 1_717_000_000_001,
                ..event("corr-2", all_hook_kinds().remove(1))
            }),
        ),
        (
            include_str!("fixtures/post_tool_use.json"),
            SubscriptionFrame::Event(HookEvent {
                ts: 1_717_000_000_002,
                ..event("corr-3", all_hook_kinds().remove(2))
            }),
        ),
        (
            include_str!("fixtures/permission_request.json"),
            SubscriptionFrame::Event(HookEvent {
                ts: 1_717_000_000_003,
                ..event("corr-4", all_hook_kinds().remove(3))
            }),
        ),
        (
            include_str!("fixtures/stop.json"),
            SubscriptionFrame::Event(HookEvent {
                ts: 1_717_000_000_004,
                ..event("corr-5", all_hook_kinds().remove(4))
            }),
        ),
        (
            include_str!("fixtures/session_end.json"),
            SubscriptionFrame::Event(HookEvent {
                ts: 1_717_000_000_005,
                ..event("corr-6", all_hook_kinds().remove(5))
            }),
        ),
    ];

    for (payload, expected) in cases {
        assert_eq!(decode_subscription_frame(&raw(payload)), Some(expected));
    }
}

#[test]
fn negotiate_ready_accepts_matching_wire_version() {
    assert_eq!(
        negotiate_ready(&SubscriptionFrame::Ready {
            wire_version: WIRE_VERSION
        }),
        Ok(WIRE_VERSION)
    );
}

#[test]
fn negotiate_ready_rejects_mismatched_wire_version() {
    assert_eq!(
        negotiate_ready(&SubscriptionFrame::Ready {
            wire_version: WIRE_VERSION + 1
        }),
        Err(DecodeError::WireVersionMismatch {
            expected: WIRE_VERSION,
            got: WIRE_VERSION + 1,
        })
    );
}

#[test]
fn negotiate_ready_surfaces_gate_error_reason() {
    assert_eq!(
        negotiate_ready(&SubscriptionFrame::Error {
            reason: "unsupported wire version".into()
        }),
        Err(DecodeError::Rejected {
            reason: "unsupported wire version".into()
        })
    );
}
