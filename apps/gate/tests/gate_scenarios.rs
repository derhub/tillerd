//! Scenario tests: 1-to-1 mapping. Network tests #[ignore]d.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use contracts::{CorrelationId, HookEvent, HookKind, SessionId};
use tillerd_gate::agent_adapter::{AgentAdapter, ParseError, V1Adapter};
use tillerd_gate::middleware::auth::Auth;
use tillerd_gate::middleware::fanout::FanOut;
use tillerd_gate::middleware::normalize::Normalize;
use tillerd_gate::middleware::observe::{ObservationRecord, Observe, ObserveSink};
use tillerd_gate::middleware::passthrough::PassThrough;
use tillerd_gate::middleware::{seq, Middleware};
use tillerd_gate::registry::SessionRegistry;
use tillerd_gate::router::{Inbound, Router};
use tillerd_gate::subscription::Subscriptions;
use tillerd_gate::{Kind, Outbound, Reject, Token};

// -- helpers ------------------------------------------------------------------

fn stop_body() -> Bytes {
    Bytes::from_static(br#"{"hook_event_name":"Stop","session_id":"agent","timestamp_ms":5}"#)
}

fn build_router(registry: Arc<SessionRegistry>, subscriptions: Arc<Subscriptions>) -> Router {
    let globals = vec![Arc::new(Auth::new(registry)) as Arc<dyn Middleware>];
    let hook_route = seq(vec![
        Arc::new(Normalize::new(Arc::new(V1Adapter))) as Arc<dyn Middleware>,
        Arc::new(FanOut::new(subscriptions)),
    ]);
    let routes = HashMap::from([
        (Kind::Hook, hook_route),
        (Kind::ToolCall, Arc::new(PassThrough) as Arc<dyn Middleware>),
        (
            Kind::ToolResult,
            Arc::new(PassThrough) as Arc<dyn Middleware>,
        ),
    ]);
    Router::new(globals, routes)
}

// -- Gate carries no tool-call knowledge --------------------------------------

/// The gate's tool route contains no backend registry: it is a pure pass-through
/// that returns the caller's payload unchanged. The caller, not the gate, routes
/// tool results to the right backend.
#[tokio::test]
async fn gate_carries_no_tool_call_knowledge_tool_route_is_passthrough() {
    let registry = Arc::new(SessionRegistry::new());
    let session = SessionId("s".into());
    registry.register(session.clone(), &Token::new("tok"));
    let router = build_router(registry, Arc::new(Subscriptions::with_capacity(8)));

    let payload = Bytes::from_static(b"tool-payload-opaque");
    let out = router
        .handle(Inbound {
            kind: Kind::ToolCall,
            session: session.clone(),
            correlation: Some(CorrelationId("c".into())),
            token: Token::new("tok"),
            body: payload.clone(),
        })
        .await
        .unwrap();

    // The gate forwards the body without interpreting it -- no backend registry,
    // no protocol awareness.
    assert_eq!(
        out,
        Outbound::Forward(payload),
        "tool route returns the body unchanged: no backend registry in the gate"
    );
}

// -- Normalization uses the injected adapter -----------------------------------

/// Replacing the adapter changes what `Normalize` parses; the gate itself
/// contains no agent-specific logic.
#[tokio::test]
async fn normalization_uses_the_injected_adapter_not_hard_coded_logic() {
    struct FixedAdapter;
    impl AgentAdapter for FixedAdapter {
        fn parse_hook(&self, _body: &[u8]) -> Result<HookEvent, ParseError> {
            Ok(HookEvent {
                session_id: SessionId("injected".into()),
                correlation_id: CorrelationId("injected-corr".into()),
                ts: 42,
                kind: HookKind::SessionStart {
                    cwd: Some("/fixed".into()),
                    client: Some("custom-adapter".into()),
                    cli_version: None,
                },
            })
        }
    }

    let subscriptions = Arc::new(Subscriptions::with_capacity(8));
    let session = SessionId("s".into());
    let mut rx = subscriptions.subscribe(&session);

    let route = seq(vec![
        Arc::new(Normalize::new(Arc::new(FixedAdapter))) as Arc<dyn Middleware>,
        Arc::new(FanOut::new(subscriptions)),
    ]);
    let router = Router::new(vec![], HashMap::from([(Kind::Hook, route)]));

    router
        .handle(Inbound {
            kind: Kind::Hook,
            session: session.clone(),
            correlation: Some(CorrelationId("corr".into())),
            token: Token::new("t"),
            body: Bytes::from_static(b"{}"),
        })
        .await
        .unwrap();

    let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("event delivered")
        .unwrap();

    assert_eq!(
        event.kind,
        HookKind::SessionStart {
            cwd: Some("/fixed".into()),
            client: Some("custom-adapter".into()),
            cli_version: None,
        },
        "the normalized event reflects the injected adapter, not V1Adapter"
    );
}

// -- Gate is agent-agnostic in code -------------------------------------------

/// Swapping to a different adapter (ErrAdapter -- simulates an unsupported agent)
/// rejects unknown payloads without any gate code change.
#[tokio::test]
async fn gate_is_agent_agnostic_only_the_adapter_changes() {
    struct RejectAllAdapter;
    impl AgentAdapter for RejectAllAdapter {
        fn parse_hook(&self, _body: &[u8]) -> Result<HookEvent, ParseError> {
            Err(ParseError::MissingField("unknown-agent-field".into()))
        }
    }

    let route = seq(vec![
        Arc::new(Normalize::new(Arc::new(RejectAllAdapter))) as Arc<dyn Middleware>
    ]);
    let router = Router::new(vec![], HashMap::from([(Kind::Hook, route)]));

    let err = router
        .handle(Inbound {
            kind: Kind::Hook,
            session: SessionId("s".into()),
            correlation: None,
            token: Token::new("t"),
            body: Bytes::from_static(b"{\"unknown\":1}"),
        })
        .await
        .unwrap_err();

    assert!(
        matches!(err, Reject::Invalid(_)),
        "swapping to a rejecting adapter rejects without touching the gate"
    );
}

// -- Agent surfaces cannot administer -----------------------------------------

/// A caller on the tool route cannot cause a session to be registered; the admin
/// surface is the only route that reaches the registry for writes.
#[tokio::test]
async fn tool_route_caller_cannot_register_a_session() {
    let registry = Arc::new(SessionRegistry::new());
    let session = SessionId("victim".into());
    registry.register(session.clone(), &Token::new("real-token"));

    let router = build_router(registry.clone(), Arc::new(Subscriptions::with_capacity(8)));

    // Attempt: send a tool call that looks like it might register a new session.
    let _ = router
        .handle(Inbound {
            kind: Kind::ToolCall,
            session: session.clone(),
            correlation: None,
            token: Token::new("real-token"),
            body: Bytes::from_static(
                br#"{"command":"register","sessionId":"intruder","token":"x"}"#,
            ),
        })
        .await;

    // The intruder session must not exist in the registry.
    assert!(
        registry
            .verify(&SessionId("intruder".into()), &Token::new("x"))
            .is_none(),
        "the tool route has no path to the session registry for writes"
    );
}

/// A caller on the hook endpoint cannot register a session via the hook body.
/// This mirrors the face-isolation requirement.
#[tokio::test]
async fn hook_caller_cannot_register_a_session_via_hook_body() {
    let registry = Arc::new(SessionRegistry::new());
    let session = SessionId("s".into());
    registry.register(session.clone(), &Token::new("tok"));

    let router = build_router(registry.clone(), Arc::new(Subscriptions::with_capacity(8)));

    // A hook body that looks like a register command: must not reach the admin
    // face, which is the only surface authorised to mutate the registry.
    let result = router
        .handle(Inbound {
            kind: Kind::Hook,
            session: session.clone(),
            correlation: None,
            token: Token::new("tok"),
            body: Bytes::from_static(
                br#"{"command":"register","sessionId":"injected","token":"y"}"#,
            ),
        })
        .await;

    // A hook body with no `hook_event_name` should be rejected as invalid, not
    // treated as an admin command.
    assert!(
        result.is_err(),
        "a hook body cannot be interpreted as an admin command"
    );
    assert!(
        registry
            .verify(&SessionId("injected".into()), &Token::new("y"))
            .is_none(),
        "the hook face cannot create a registry entry via the hook body"
    );
}

// -- Observe wraps auth so rejections are recorded ----------------------------

/// When globals are `[Observe, Auth]`, an auth rejection is still recorded by
/// the outermost Observe layer -- this is the "inbound runs globals then its route"
/// scenario asserting correct ordering of globals.
#[derive(Default)]
struct CaptureSink {
    records: Mutex<Vec<ObservationRecord>>,
}

impl ObserveSink for CaptureSink {
    fn emit(&self, r: ObservationRecord) {
        self.records.lock().unwrap().push(r);
    }
}

#[tokio::test]
async fn observe_is_outermost_so_it_records_an_auth_rejection() {
    let sink = Arc::new(CaptureSink::default());
    let registry = Arc::new(SessionRegistry::new());
    let globals = vec![
        Arc::new(Observe::new(sink.clone())) as Arc<dyn Middleware>,
        Arc::new(Auth::new(registry)),
    ];
    let router = Router::new(
        globals,
        HashMap::from([(Kind::Hook, Arc::new(PassThrough) as Arc<dyn Middleware>)]),
    );

    let err = router
        .handle(Inbound {
            kind: Kind::Hook,
            session: SessionId("no-session".into()),
            correlation: None,
            token: Token::new("bad"),
            body: stop_body(),
        })
        .await
        .unwrap_err();

    assert_eq!(err, Reject::Unauthenticated);
    let records = sink.records.lock().unwrap();
    assert_eq!(
        records.len(),
        1,
        "one record per inbound, even for rejections"
    );
}
