//! Observe: exactly one record per inbound (outer layer, outermost global).

use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use contracts::{CorrelationId, HookKind, SessionId};

use crate::middleware::{Middleware, Next};
use crate::{Ctx, Flow, Kind};

/// The component identity carried on every record this gate emits.
const COMPONENT: &str = "gate";

/// Whether an observed inbound was accepted or rejected (with the reason).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordOutcome {
    /// The inner chain returned `Ok`.
    Accepted,
    /// The inner chain returned `Err`; the string is the rejection reason.
    Rejected(String),
}

/// One observation record emitted per inbound the gate handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationRecord {
    /// Epoch-millisecond timestamp at the point the record was emitted.
    pub ts: i64,
    /// The session the inbound was attributed to.
    pub session_id: SessionId,
    /// The correlation id assigned to this inbound.
    pub correlation_id: CorrelationId,
    /// The gate component identity, always `"gate"`.
    pub component: &'static str,
    /// The inbound kind (hook, tool-call, or tool-result).
    pub kind: Kind,
    /// The canonical event-type name, set by the normalize layer for hook inbounds.
    pub event_type: Option<String>,
    /// Whether the inbound was accepted or rejected, and the rejection reason.
    pub outcome: RecordOutcome,
    /// Wall-time latency of the inner chain in milliseconds.
    pub latency_ms: u64,
    /// Number of subscribers reached by the fan-out layer, if applicable.
    pub fanout_n: Option<u32>,
    /// Number of events dropped (lag) observed on this session's channel.
    pub dropped_n: Option<u64>,
}

/// The destination for observation records. The production sink logs them; tests
/// capture them.
pub trait ObserveSink: Send + Sync {
    /// Receive one observation record for an inbound the gate has finished handling.
    fn emit(&self, record: ObservationRecord);
}

/// Times the inner chain and emits one record per inbound to its sink.
pub struct Observe {
    sink: Arc<dyn ObserveSink>,
}

impl Observe {
    /// Build an observe layer that emits records to the given sink.
    pub fn new(sink: Arc<dyn ObserveSink>) -> Self {
        Self { sink }
    }
}

#[async_trait]
impl Middleware for Observe {
    async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
        let session_id = ctx.session.clone();
        let correlation_id = ctx.correlation.clone();
        let kind = ctx.kind;
        // The inner chain fills these only after ctx moves into next; the shared
        // handle survives the move so they can be read once it returns.
        let record = ctx.record.clone();

        let started = Instant::now();
        let flow = next.run(ctx).await;
        let latency_ms = started.elapsed().as_millis() as u64;

        let outcome = match &flow {
            Ok(_) => RecordOutcome::Accepted,
            Err(reject) => RecordOutcome::Rejected(reject.to_string()),
        };

        let (event_type, fanout_n) = {
            let meta = record.lock().expect("record meta mutex poisoned");
            (meta.event_type.clone(), meta.fanout_n.map(|n| n as u32))
        };

        self.sink.emit(ObservationRecord {
            ts: now_ms(),
            session_id,
            correlation_id,
            component: COMPONENT,
            kind,
            event_type,
            outcome,
            latency_ms,
            fanout_n,
            dropped_n: None,
        });

        flow
    }
}

pub(crate) fn event_type_name(kind: &HookKind) -> String {
    match kind {
        HookKind::SessionStart { .. } => "SessionStart",
        HookKind::UserPromptSubmit { .. } => "UserPromptSubmit",
        HookKind::PostToolUse { .. } => "PostToolUse",
        HookKind::PermissionRequest { .. } => "PermissionRequest",
        HookKind::Stop { .. } => "Stop",
        HookKind::SessionEnd { .. } => "SessionEnd",
    }
    .to_string()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_adapter::V1Adapter;
    use crate::middleware::auth::Auth;
    use crate::middleware::fanout::FanOut;
    use crate::middleware::normalize::Normalize;
    use crate::middleware::seq;
    use crate::registry::SessionRegistry;
    use crate::subscription::Subscriptions;
    use crate::{Outbound, Reject, Token};
    use bytes::Bytes;
    use std::sync::Mutex;
    use std::time::Duration;

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
        fn only(&self) -> ObservationRecord {
            let records = self.records.lock().unwrap();
            assert_eq!(records.len(), 1, "exactly one record per inbound");
            records[0].clone()
        }
    }

    fn ctx() -> Ctx {
        Ctx {
            kind: Kind::Hook,
            session: SessionId("s1".into()),
            correlation: CorrelationId("corr-1".into()),
            token: Token::new("t"),
            body: Bytes::new(),
            event: None,
            record: Default::default(),
        }
    }

    struct Slow(Duration);

    #[async_trait]
    impl Middleware for Slow {
        async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
            tokio::time::sleep(self.0).await;
            next.run(ctx).await
        }
    }

    struct Failing;

    #[async_trait]
    impl Middleware for Failing {
        async fn handle(&self, _ctx: Ctx, _next: Next<'_>) -> Flow {
            Err(Reject::Invalid("bad".into()))
        }
    }

    #[tokio::test]
    async fn emits_an_accepted_record_on_success() {
        let sink = Arc::new(FakeSink::default());
        let observe = Observe::new(sink.clone());

        let out = observe.handle(ctx(), Next::noop()).await.unwrap();

        assert_eq!(out, Outbound::Accepted);
        assert_eq!(sink.only().outcome, RecordOutcome::Accepted);
    }

    #[tokio::test]
    async fn emits_a_rejected_record_with_the_reason_on_failure() {
        let sink = Arc::new(FakeSink::default());
        let chain = seq(vec![
            Arc::new(Observe::new(sink.clone())),
            Arc::new(Failing),
        ]);

        let err = chain.handle(ctx(), Next::noop()).await.unwrap_err();

        assert_eq!(err, Reject::Invalid("bad".into()));
        assert_eq!(
            sink.only().outcome,
            RecordOutcome::Rejected("invalid: bad".into())
        );
    }

    #[tokio::test]
    async fn records_the_latency_of_the_inner_chain() {
        let sink = Arc::new(FakeSink::default());
        let chain = seq(vec![
            Arc::new(Observe::new(sink.clone())),
            Arc::new(Slow(Duration::from_millis(15))),
        ]);

        chain.handle(ctx(), Next::noop()).await.unwrap();

        assert!(
            sink.only().latency_ms >= 10,
            "latency reflects the inner chain duration"
        );
    }

    #[tokio::test]
    async fn binds_session_correlation_and_component() {
        let sink = Arc::new(FakeSink::default());
        let observe = Observe::new(sink.clone());

        observe.handle(ctx(), Next::noop()).await.unwrap();

        let record = sink.only();
        assert_eq!(record.session_id, SessionId("s1".into()));
        assert_eq!(record.correlation_id, CorrelationId("corr-1".into()));
        assert_eq!(record.component, "gate");
    }

    #[tokio::test]
    async fn records_an_auth_rejection_from_the_outermost_layer() {
        let sink = Arc::new(FakeSink::default());
        let chain = seq(vec![
            Arc::new(Observe::new(sink.clone())),
            Arc::new(Auth::new(Arc::new(SessionRegistry::new()))),
        ]);

        let err = chain.handle(ctx(), Next::noop()).await.unwrap_err();

        assert_eq!(err, Reject::Unauthenticated);
        assert_eq!(
            sink.only().outcome,
            RecordOutcome::Rejected("unauthenticated".into())
        );
    }

    #[tokio::test]
    async fn records_the_event_type_and_fanout_count_from_the_inner_chain() {
        // The downstream Normalize/FanOut layers learn the event type and reach
        // only after Observe has moved ctx into next; the back-channel must carry
        // them back so the record is populated rather than null.
        let sink = Arc::new(FakeSink::default());
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let _rx = subscriptions.subscribe(&SessionId("s1".into()));
        let chain = seq(vec![
            Arc::new(Observe::new(sink.clone())) as Arc<dyn Middleware>,
            Arc::new(Normalize::new(Arc::new(V1Adapter))),
            Arc::new(FanOut::new(subscriptions)),
        ]);
        let mut ctx = ctx();
        ctx.body = Bytes::from_static(br#"{"hook_event_name":"Stop","session_id":"agent"}"#);

        chain.handle(ctx, Next::noop()).await.unwrap();

        let record = sink.only();
        assert_eq!(record.event_type, Some("Stop".into()));
        assert_eq!(record.fanout_n, Some(1));
    }
}
