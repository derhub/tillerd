//! Router integration: face isolation, auth recorded, teardown on end.
//! correlation preservation from inbound to event to record.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use contracts::{CorrelationId, HookKind, SessionId};
use tillerd_gate::agent_adapter::V1Adapter;
use tillerd_gate::middleware::auth::Auth;
use tillerd_gate::middleware::fanout::FanOut;
use tillerd_gate::middleware::normalize::Normalize;
use tillerd_gate::middleware::observe::{ObservationRecord, Observe, ObserveSink, RecordOutcome};
use tillerd_gate::middleware::passthrough::PassThrough;
use tillerd_gate::middleware::{seq, Middleware};
use tillerd_gate::registry::SessionRegistry;
use tillerd_gate::router::{Inbound, Router};
use tillerd_gate::subscription::Subscriptions;
use tillerd_gate::{Kind, Outbound, Reject, Token};
use tokio::sync::broadcast::error::{RecvError, TryRecvError};

#[derive(Default)]
struct FakeSink {
    records: Mutex<Vec<ObservationRecord>>,
}

impl ObserveSink for FakeSink {
    fn emit(&self, record: ObservationRecord) {
        self.records.lock().unwrap().push(record);
    }
}

impl FakeSink {
    fn records(&self) -> Vec<ObservationRecord> {
        self.records.lock().unwrap().clone()
    }
}

struct Harness {
    router: Router,
    registry: Arc<SessionRegistry>,
    subscriptions: Arc<Subscriptions>,
    sink: Arc<FakeSink>,
}

/// The production wiring: globals `[Observe, Auth]` (Observe outermost) around a
/// `Hook` route of `Normalize -> FanOut` and `PassThrough` tool routes.
fn harness() -> Harness {
    let registry = Arc::new(SessionRegistry::new());
    let subscriptions = Arc::new(Subscriptions::with_capacity(16));
    let sink = Arc::new(FakeSink::default());
    let globals = vec![
        Arc::new(Observe::new(sink.clone())) as Arc<dyn Middleware>,
        Arc::new(Auth::new(registry.clone())),
    ];
    let hook_route = seq(vec![
        Arc::new(Normalize::new(Arc::new(V1Adapter))) as Arc<dyn Middleware>,
        Arc::new(FanOut::new(subscriptions.clone())),
    ]);
    let routes = HashMap::from([
        (Kind::Hook, hook_route),
        (Kind::ToolCall, Arc::new(PassThrough) as Arc<dyn Middleware>),
        (
            Kind::ToolResult,
            Arc::new(PassThrough) as Arc<dyn Middleware>,
        ),
    ]);
    Harness {
        router: Router::new(globals, routes),
        registry,
        subscriptions,
        sink,
    }
}

fn stop_hook() -> Bytes {
    Bytes::from_static(br#"{"hook_event_name":"Stop","session_id":"agent","timestamp_ms":5}"#)
}

#[tokio::test]
async fn a_hook_fans_out_to_every_subscriber() {
    let h = harness();
    let session = SessionId("s".into());
    h.registry.register(session.clone(), &Token::new("t"));
    let mut a = h.subscriptions.subscribe(&session);
    let mut b = h.subscriptions.subscribe(&session);
    let mut c = h.subscriptions.subscribe(&session);

    let out = h
        .router
        .handle(Inbound {
            kind: Kind::Hook,
            session: session.clone(),
            correlation: None,
            token: Token::new("t"),
            body: stop_hook(),
        })
        .await
        .unwrap();

    assert_eq!(out, Outbound::Accepted);
    for rx in [&mut a, &mut b, &mut c] {
        let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("each subscriber receives the fanned-out event")
            .unwrap();
        assert_eq!(event.kind, HookKind::Stop { turn_index: None });
    }
}

#[tokio::test]
async fn a_tool_call_and_result_are_forwarded_unchanged() {
    let h = harness();
    let session = SessionId("s".into());
    h.registry.register(session.clone(), &Token::new("t"));

    for kind in [Kind::ToolCall, Kind::ToolResult] {
        let body = Bytes::from_static(b"tool-payload");
        let out = h
            .router
            .handle(Inbound {
                kind,
                session: session.clone(),
                correlation: Some(CorrelationId("c".into())),
                token: Token::new("t"),
                body: body.clone(),
            })
            .await
            .unwrap();

        assert_eq!(out, Outbound::Forward(body));
    }
}

#[tokio::test]
async fn an_unauthenticated_inbound_is_rejected_and_still_recorded() {
    let h = harness();

    let err = h
        .router
        .handle(Inbound {
            kind: Kind::Hook,
            session: SessionId("ghost".into()),
            correlation: None,
            token: Token::new("nope"),
            body: stop_hook(),
        })
        .await
        .unwrap_err();

    assert_eq!(err, Reject::Unauthenticated);
    let records = h.sink.records();
    assert_eq!(records.len(), 1, "the outermost observe layer records once");
    assert_eq!(
        records[0].outcome,
        RecordOutcome::Rejected("unauthenticated".into())
    );
}

#[tokio::test]
async fn a_tool_call_never_publishes_to_hook_subscribers() {
    let h = harness();
    let session = SessionId("s".into());
    h.registry.register(session.clone(), &Token::new("t"));
    let mut rx = h.subscriptions.subscribe(&session);

    let out = h
        .router
        .handle(Inbound {
            kind: Kind::ToolCall,
            session: session.clone(),
            correlation: Some(CorrelationId("c".into())),
            token: Token::new("t"),
            body: Bytes::from_static(b"{}"),
        })
        .await
        .unwrap();

    assert!(matches!(out, Outbound::Forward(_)));
    assert!(
        matches!(rx.try_recv(), Err(TryRecvError::Empty)),
        "the tool route has no path to the hook subscribers"
    );
}

#[tokio::test]
async fn a_hook_body_cannot_register_a_session() {
    let h = harness();
    let victim = SessionId("victim".into());
    h.registry.register(victim.clone(), &Token::new("vt"));

    let flow = h
        .router
        .handle(Inbound {
            kind: Kind::Hook,
            session: victim,
            correlation: None,
            token: Token::new("vt"),
            body: Bytes::from_static(
                br#"{"command":"register","sessionId":"intruder","token":"x"}"#,
            ),
        })
        .await;

    assert!(matches!(flow, Err(Reject::Invalid(_))));
    assert!(
        h.registry
            .verify(&SessionId("intruder".into()), &Token::new("x"))
            .is_none(),
        "the hook face cannot create a registry entry"
    );
}

#[tokio::test]
async fn ending_a_session_tears_down_its_subscription() {
    let h = harness();
    let session = SessionId("s".into());
    let mut rx = h.subscriptions.subscribe(&session);

    h.subscriptions.end(&session);

    assert!(matches!(rx.recv().await, Err(RecvError::Closed)));
}

#[tokio::test]
async fn a_supplied_correlation_is_preserved_into_the_event_and_the_record() {
    let h = harness();
    let session = SessionId("s".into());
    h.registry.register(session.clone(), &Token::new("t"));
    let mut rx = h.subscriptions.subscribe(&session);

    h.router
        .handle(Inbound {
            kind: Kind::Hook,
            session: session.clone(),
            correlation: Some(CorrelationId("corr-xyz".into())),
            token: Token::new("t"),
            body: stop_hook(),
        })
        .await
        .unwrap();

    let event = rx.recv().await.unwrap();
    assert_eq!(event.correlation_id, CorrelationId("corr-xyz".into()));
    assert_eq!(
        h.sink.records()[0].correlation_id,
        CorrelationId("corr-xyz".into()),
        "the same correlation id reaches the observation record"
    );
}
