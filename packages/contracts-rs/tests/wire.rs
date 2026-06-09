use contracts::{
    CorrelationId, HookEvent, HookKind, Route, RoutePreamble, SessionId, ToolInbound,
    HOOK_SUBSCRIPTION_WIRE_VERSION, SESSION_EVENT_WIRE_VERSION,
};
use serde_json::{json, Value};

#[test]
fn route_preamble_round_trips_with_camelcase_keys() {
    let preamble = RoutePreamble {
        route: Route::Hook,
        session_id: Some(SessionId("s1".into())),
        token: Some("tok".into()),
        wire_version: HOOK_SUBSCRIPTION_WIRE_VERSION,
    };

    let wire = serde_json::to_value(&preamble).expect("encode");
    assert_eq!(
        wire,
        json!({
            "route": "hook",
            "sessionId": "s1",
            "token": "tok",
            "wireVersion": HOOK_SUBSCRIPTION_WIRE_VERSION,
        })
    );
    assert_eq!(
        serde_json::from_value::<RoutePreamble>(wire).expect("decode"),
        preamble
    );
}

#[test]
fn route_preamble_omits_absent_session_and_token() {
    let preamble = RoutePreamble {
        route: Route::Admin,
        session_id: None,
        token: Some("admin".into()),
        wire_version: HOOK_SUBSCRIPTION_WIRE_VERSION,
    };

    assert_eq!(
        serde_json::to_value(&preamble).expect("encode"),
        json!({ "route": "admin", "token": "admin", "wireVersion": HOOK_SUBSCRIPTION_WIRE_VERSION })
    );
}

#[test]
fn route_enum_serializes_camelcase() {
    assert_eq!(
        serde_json::to_value(Route::Subscribe).unwrap(),
        json!("subscribe")
    );
    assert_eq!(serde_json::to_value(Route::Mcp).unwrap(), json!("mcp"));
}

fn fixtures() -> Vec<Value> {
    let raw = include_str!("../fixtures/hook_events.json");
    serde_json::from_str(raw).expect("fixtures parse")
}

#[test]
fn hook_event_serializes_flat_with_camelcase_keys() {
    let event = HookEvent {
        session_id: SessionId("s1".into()),
        correlation_id: CorrelationId("c1".into()),
        ts: 1_700_000_000_001,
        kind: HookKind::UserPromptSubmit {
            content: "hello".into(),
            turn_index: Some(0),
        },
    };

    assert_eq!(
        serde_json::to_value(&event).expect("encode"),
        json!({
            "sessionId": "s1",
            "correlationId": "c1",
            "ts": 1_700_000_000_001i64,
            "type": "UserPromptSubmit",
            "payload": { "content": "hello", "turnIndex": 0 }
        })
    );
}

#[test]
fn absent_optional_payload_fields_are_omitted() {
    let event = HookEvent {
        session_id: SessionId("s1".into()),
        correlation_id: CorrelationId("c1".into()),
        ts: 1,
        kind: HookKind::Stop { turn_index: None },
    };

    assert_eq!(
        serde_json::to_value(&event).expect("encode"),
        json!({
            "sessionId": "s1",
            "correlationId": "c1",
            "ts": 1,
            "type": "Stop",
            "payload": {}
        })
    );
}

#[test]
fn every_hook_event_fixture_round_trips() {
    for fixture in fixtures() {
        let event: HookEvent = serde_json::from_value(fixture.clone()).expect("decode fixture");
        assert_eq!(serde_json::to_value(&event).expect("encode"), fixture);
    }
}

#[test]
fn tool_inbound_round_trips() {
    let inbound = ToolInbound::ToolCall {
        session_id: SessionId("s1".into()),
        correlation_id: CorrelationId("c1".into()),
        tool_name: "Bash".into(),
        tool_input: json!({ "command": "ls" }),
    };
    let decoded: ToolInbound =
        serde_json::from_value(serde_json::to_value(&inbound).expect("encode")).expect("decode");
    assert_eq!(decoded, inbound);
}

#[test]
fn session_and_hook_subscription_wires_are_versioned_independently() {
    // R9: the daemon session-event wire and the gate hook-subscription wire carry
    // their own version constants rather than a shared one.
    assert_eq!(
        (SESSION_EVENT_WIRE_VERSION, HOOK_SUBSCRIPTION_WIRE_VERSION),
        (1, 1)
    );
}
