//! The router: the single entry point every transport face funnels inbounds
//! through. It assigns the correlation id exactly once (when absent), then runs
//! the global onion (observe, auth) wrapped around the route chosen by kind.

use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use contracts::{CorrelationId, SessionId};
use uuid::Uuid;

use crate::middleware::{seq, Middleware, Next};
use crate::{Ctx, Flow, Kind, Reject, Token};

/// An untrusted inbound from a transport face. Its correlation id is optional;
/// the router assigns one when absent.
pub struct Inbound {
    /// The inbound kind, used to select the route.
    pub kind: Kind,
    /// The session this inbound is attributed to.
    pub session: SessionId,
    /// Caller-supplied correlation id; the router mints a fresh UUID when absent.
    pub correlation: Option<CorrelationId>,
    /// The bearer token extracted by the transport face.
    pub token: Token,
    /// The raw payload as received from the transport face.
    pub body: Bytes,
}

/// Dispatches inbounds through the global onion and a per-kind route.
pub struct Router {
    chains: HashMap<Kind, Arc<dyn Middleware>>,
}

impl Router {
    /// Build a router from the globals (outermost first) and the per-kind routes.
    /// Each kind's full chain is the globals wrapped around its route.
    pub fn new(
        globals: Vec<Arc<dyn Middleware>>,
        routes: HashMap<Kind, Arc<dyn Middleware>>,
    ) -> Self {
        let chains = routes
            .into_iter()
            .map(|(kind, route)| {
                let mut items = globals.clone();
                items.push(route);
                (kind, seq(items))
            })
            .collect();
        Self { chains }
    }

    /// Handle one inbound: assign the correlation id once (when absent) — the
    /// sole point it is minted — then run the kind's chain.
    pub async fn handle(&self, inbound: Inbound) -> Flow {
        let Inbound {
            kind,
            session,
            correlation,
            token,
            body,
        } = inbound;
        let correlation = correlation.unwrap_or_else(|| CorrelationId(Uuid::new_v4().to_string()));
        let Some(chain) = self.chains.get(&kind) else {
            return Err(Reject::Invalid(format!("no route for {kind:?}")));
        };
        let ctx = Ctx {
            kind,
            session,
            correlation,
            token,
            body,
            event: None,
            record: Default::default(),
        };
        chain.handle(ctx, Next::noop()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_adapter::V1Adapter;
    use crate::middleware::auth::Auth;
    use crate::middleware::fanout::FanOut;
    use crate::middleware::normalize::Normalize;
    use crate::middleware::observe::{ObservationRecord, Observe, ObserveSink, RecordOutcome};
    use crate::registry::SessionRegistry;
    use crate::subscription::Subscriptions;
    use crate::Outbound;
    use async_trait::async_trait;
    use std::sync::Mutex;

    fn inbound(kind: Kind) -> Inbound {
        Inbound {
            kind,
            session: SessionId("s".into()),
            correlation: None,
            token: Token::new("t"),
            body: Bytes::new(),
        }
    }

    struct Recorder {
        label: &'static str,
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    #[async_trait]
    impl Middleware for Recorder {
        async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
            self.log.lock().unwrap().push(self.label);
            next.run(ctx).await
        }
    }

    fn recorder(label: &'static str, log: Arc<Mutex<Vec<&'static str>>>) -> Arc<dyn Middleware> {
        Arc::new(Recorder { label, log })
    }

    struct SeeCorrelation {
        seen: Arc<Mutex<Option<String>>>,
    }

    #[async_trait]
    impl Middleware for SeeCorrelation {
        async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
            *self.seen.lock().unwrap() = Some(ctx.correlation.0.clone());
            next.run(ctx).await
        }
    }

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

    #[tokio::test]
    async fn dispatches_to_the_route_matching_the_kind() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let routes = HashMap::from([
            (Kind::Hook, recorder("hook", log.clone())),
            (Kind::ToolCall, recorder("tool", log.clone())),
        ]);
        let router = Router::new(vec![], routes);

        router.handle(inbound(Kind::Hook)).await.unwrap();
        router.handle(inbound(Kind::ToolCall)).await.unwrap();

        assert_eq!(*log.lock().unwrap(), vec!["hook", "tool"]);
    }

    #[tokio::test]
    async fn runs_globals_outermost_so_observe_records_an_auth_rejection() {
        let sink = Arc::new(FakeSink::default());
        let route_log = Arc::new(Mutex::new(Vec::new()));
        let globals = vec![
            Arc::new(Observe::new(sink.clone())) as Arc<dyn Middleware>,
            Arc::new(Auth::new(Arc::new(SessionRegistry::new()))),
        ];
        let routes = HashMap::from([(Kind::Hook, recorder("route", route_log.clone()))]);
        let router = Router::new(globals, routes);

        let err = router.handle(inbound(Kind::Hook)).await.unwrap_err();

        assert_eq!(err, Reject::Unauthenticated);
        assert!(
            route_log.lock().unwrap().is_empty(),
            "an auth rejection stops before the route runs"
        );
        assert_eq!(
            sink.only().outcome,
            RecordOutcome::Rejected("unauthenticated".into()),
            "the outermost observe layer still records the rejection"
        );
    }

    #[tokio::test]
    async fn preserves_a_supplied_correlation_id() {
        let seen = Arc::new(Mutex::new(None));
        let routes = HashMap::from([(
            Kind::Hook,
            Arc::new(SeeCorrelation { seen: seen.clone() }) as Arc<dyn Middleware>,
        )]);
        let router = Router::new(vec![], routes);
        let mut inbound = inbound(Kind::Hook);
        inbound.correlation = Some(CorrelationId("supplied".into()));

        router.handle(inbound).await.unwrap();

        assert_eq!(seen.lock().unwrap().as_deref(), Some("supplied"));
    }

    #[tokio::test]
    async fn assigns_a_fresh_correlation_id_when_absent() {
        let seen = Arc::new(Mutex::new(None));
        let routes = HashMap::from([(
            Kind::Hook,
            Arc::new(SeeCorrelation { seen: seen.clone() }) as Arc<dyn Middleware>,
        )]);
        let router = Router::new(vec![], routes);

        router.handle(inbound(Kind::Hook)).await.unwrap();

        let assigned = seen
            .lock()
            .unwrap()
            .clone()
            .expect("a correlation was seen");
        assert!(
            Uuid::parse_str(&assigned).is_ok(),
            "an absent correlation is assigned a fresh uuid"
        );
    }

    #[tokio::test]
    async fn stamps_a_gate_timestamp_when_the_hook_omits_one() {
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let session = SessionId("s".into());
        let mut rx = subscriptions.subscribe(&session);
        let route = seq(vec![
            Arc::new(Normalize::new(Arc::new(V1Adapter))) as Arc<dyn Middleware>,
            Arc::new(FanOut::new(subscriptions)),
        ]);
        let routes = HashMap::from([(Kind::Hook, route)]);
        let router = Router::new(vec![], routes);

        router
            .handle(Inbound {
                kind: Kind::Hook,
                session,
                correlation: None,
                token: Token::new("t"),
                body: Bytes::from_static(br#"{"hook_event_name":"Stop","session_id":"agent"}"#),
            })
            .await
            .unwrap();

        assert!(
            rx.recv().await.unwrap().ts > 0,
            "the gate stamps a receive-time ts when the hook omits one"
        );
    }

    #[tokio::test]
    async fn threads_the_correlation_id_into_the_event_and_the_record() {
        let sink = Arc::new(FakeSink::default());
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let session = SessionId("s".into());
        let mut rx = subscriptions.subscribe(&session);
        let route = seq(vec![
            Arc::new(Normalize::new(Arc::new(V1Adapter))),
            Arc::new(FanOut::new(subscriptions)),
        ]);
        let globals = vec![Arc::new(Observe::new(sink.clone())) as Arc<dyn Middleware>];
        let routes = HashMap::from([(Kind::Hook, route)]);
        let router = Router::new(globals, routes);

        let out = router
            .handle(Inbound {
                kind: Kind::Hook,
                session,
                correlation: Some(CorrelationId("corr-xyz".into())),
                token: Token::new("t"),
                body: Bytes::from_static(
                    br#"{"hook_event_name":"Stop","session_id":"agent","timestamp_ms":5}"#,
                ),
            })
            .await
            .unwrap();

        assert_eq!(out, Outbound::Accepted);
        assert_eq!(
            sink.only().correlation_id,
            CorrelationId("corr-xyz".into()),
            "the record carries the assigned correlation id"
        );
        assert_eq!(
            rx.recv().await.unwrap().correlation_id,
            CorrelationId("corr-xyz".into()),
            "the fanned-out event carries the same correlation id"
        );
    }
}
