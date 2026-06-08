//! The normalization layer (transform): calls the injected agent adapter to turn
//! the raw hook body into a canonical event, stamps the bound identifiers onto
//! it, and stores it on the context. A parse failure rejects without continuing.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use crate::agent_adapter::AgentAdapter;
use crate::middleware::observe::event_type_name;
use crate::middleware::{Middleware, Next};
use crate::{Ctx, Flow, Reject};

/// Supplies the receive-time epoch-millis the gate stamps when the adapter left
/// an event's timestamp unset. Injectable so tests can pin it.
type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

/// Normalizes a hook body via the injected agent adapter; stays agent-agnostic.
pub struct Normalize {
    adapter: Arc<dyn AgentAdapter>,
    clock: Clock,
}

impl Normalize {
    /// Build a normalize layer using the given agent adapter and the system clock.
    pub fn new(adapter: Arc<dyn AgentAdapter>) -> Self {
        Self {
            adapter,
            clock: Arc::new(now_ms),
        }
    }

    #[cfg(test)]
    fn with_clock(adapter: Arc<dyn AgentAdapter>, clock: Clock) -> Self {
        Self { adapter, clock }
    }
}

#[async_trait]
impl Middleware for Normalize {
    async fn handle(&self, mut ctx: Ctx, next: Next<'_>) -> Flow {
        match self.adapter.parse_hook(ctx.body.as_ref()) {
            Ok(mut event) => {
                // The authenticated session and the router-assigned correlation
                // are authoritative over anything the agent reported.
                event.session_id = ctx.session.clone();
                event.correlation_id = ctx.correlation.clone();
                // D11: the clock-free adapter leaves ts == 0 when the agent
                // reported no timestamp; the gate stamps it at receive time.
                if event.ts == 0 {
                    event.ts = (self.clock)();
                }
                ctx.record
                    .lock()
                    .expect("record meta mutex poisoned")
                    .event_type = Some(event_type_name(&event.kind));
                ctx.event = Some(event);
                next.run(ctx).await
            }
            Err(err) => Err(Reject::Invalid(err.to_string())),
        }
    }
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
    use crate::agent_adapter::{ParseError, V1Adapter};
    use crate::middleware::seq;
    use crate::{Kind, Outbound, Token};
    use bytes::Bytes;
    use contracts::{CorrelationId, HookEvent, HookKind, SessionId};
    use std::sync::Mutex;

    fn ctx(body: &[u8]) -> Ctx {
        Ctx {
            kind: Kind::Hook,
            session: SessionId("gate-session".into()),
            correlation: CorrelationId("gate-corr".into()),
            token: Token::new("t"),
            body: Bytes::copy_from_slice(body),
            event: None,
            record: Default::default(),
        }
    }

    fn stop_event(session: &str, correlation: &str) -> HookEvent {
        HookEvent {
            session_id: SessionId(session.into()),
            correlation_id: CorrelationId(correlation.into()),
            ts: 7,
            kind: HookKind::Stop { turn_index: None },
        }
    }

    struct OkAdapter(HookEvent);

    impl AgentAdapter for OkAdapter {
        fn parse_hook(&self, _body: &[u8]) -> Result<HookEvent, ParseError> {
            Ok(self.0.clone())
        }
    }

    struct ErrAdapter;

    impl AgentAdapter for ErrAdapter {
        fn parse_hook(&self, _body: &[u8]) -> Result<HookEvent, ParseError> {
            Err(ParseError::MissingField("tool_name".into()))
        }
    }

    struct SeeEvent {
        seen: Arc<Mutex<Option<HookEvent>>>,
    }

    #[async_trait]
    impl Middleware for SeeEvent {
        async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
            *self.seen.lock().unwrap() = ctx.event.clone();
            next.run(ctx).await
        }
    }

    #[tokio::test]
    async fn sets_the_event_and_continues_on_success() {
        let seen = Arc::new(Mutex::new(None));
        let chain = seq(vec![
            Arc::new(Normalize::new(Arc::new(OkAdapter(stop_event(
                "gate-session",
                "gate-corr",
            ))))),
            Arc::new(SeeEvent { seen: seen.clone() }),
        ]);

        let out = chain.handle(ctx(b"{}"), Next::noop()).await.unwrap();

        assert_eq!(out, Outbound::Accepted);
        assert!(
            seen.lock().unwrap().is_some(),
            "the normalized event reaches later layers on ctx"
        );
    }

    #[tokio::test]
    async fn stamps_ctx_session_and_correlation_onto_the_event() {
        let seen = Arc::new(Mutex::new(None));
        // The adapter reports foreign identifiers; the gate's own must win.
        let chain = seq(vec![
            Arc::new(Normalize::new(Arc::new(OkAdapter(stop_event(
                "agent-said",
                "agent-corr",
            ))))),
            Arc::new(SeeEvent { seen: seen.clone() }),
        ]);

        chain.handle(ctx(b"{}"), Next::noop()).await.unwrap();

        let event = seen.lock().unwrap().clone().unwrap();
        assert_eq!(event.session_id, SessionId("gate-session".into()));
        assert_eq!(event.correlation_id, CorrelationId("gate-corr".into()));
    }

    #[tokio::test]
    async fn stamps_the_gate_clock_when_the_adapter_leaves_ts_unset() {
        let seen = Arc::new(Mutex::new(None));
        let unstamped = HookEvent {
            ts: 0,
            ..stop_event("agent", "agent-corr")
        };
        let chain = seq(vec![
            Arc::new(Normalize::with_clock(
                Arc::new(OkAdapter(unstamped)),
                Arc::new(|| 12_345),
            )),
            Arc::new(SeeEvent { seen: seen.clone() }),
        ]);

        chain.handle(ctx(b"{}"), Next::noop()).await.unwrap();

        assert_eq!(seen.lock().unwrap().clone().unwrap().ts, 12_345);
    }

    #[tokio::test]
    async fn preserves_a_timestamp_the_adapter_already_set() {
        let seen = Arc::new(Mutex::new(None));
        // stop_event carries ts == 7; a clock that would overwrite it must not run.
        let chain = seq(vec![
            Arc::new(Normalize::with_clock(
                Arc::new(OkAdapter(stop_event("agent", "agent-corr"))),
                Arc::new(|| 12_345),
            )),
            Arc::new(SeeEvent { seen: seen.clone() }),
        ]);

        chain.handle(ctx(b"{}"), Next::noop()).await.unwrap();

        assert_eq!(seen.lock().unwrap().clone().unwrap().ts, 7);
    }

    #[tokio::test]
    async fn rejects_invalid_and_skips_next_on_parse_error() {
        let normalize = Normalize::new(Arc::new(ErrAdapter));
        let (next, called) = Next::spy();

        let err = normalize.handle(ctx(b"garbage"), next).await.unwrap_err();

        assert_eq!(err, Reject::Invalid("missing field: tool_name".into()));
        assert!(
            !*called.lock().unwrap(),
            "a parse error short-circuits without running next"
        );
    }

    #[tokio::test]
    async fn is_agent_agnostic_over_a_real_adapter() {
        let seen = Arc::new(Mutex::new(None));
        let raw =
            br#"{"hook_event_name":"Stop","session_id":"agent","timestamp_ms":5,"turn_index":2}"#;
        let chain = seq(vec![
            Arc::new(Normalize::new(Arc::new(V1Adapter))),
            Arc::new(SeeEvent { seen: seen.clone() }),
        ]);

        chain.handle(ctx(raw), Next::noop()).await.unwrap();

        let event = seen.lock().unwrap().clone().unwrap();
        assert_eq!(
            event.kind,
            HookKind::Stop {
                turn_index: Some(2)
            }
        );
        assert_eq!(event.correlation_id, CorrelationId("gate-corr".into()));
    }
}
