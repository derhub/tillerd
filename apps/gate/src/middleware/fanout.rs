//! Fanout terminal: fire-and-forget broadcast. No subscribers is normal.

use std::sync::Arc;

use async_trait::async_trait;

use crate::middleware::{Middleware, Next};
use crate::subscription::Subscriptions;
use crate::{Ctx, Flow, Outbound};

/// Publishes `ctx.event` to the session's subscribers, then accepts. Terminal:
/// it never calls `next`.
pub struct FanOut {
    subscriptions: Arc<Subscriptions>,
}

impl FanOut {
    /// Build a fan-out layer that publishes to the given subscription registry.
    pub fn new(subscriptions: Arc<Subscriptions>) -> Self {
        Self { subscriptions }
    }
}

#[async_trait]
impl Middleware for FanOut {
    async fn handle(&self, ctx: Ctx, _next: Next<'_>) -> Flow {
        if let Some(event) = ctx.event {
            // Best-effort delivery: no subscriber is a normal state, not a failure.
            if let Ok(reached) = self.subscriptions.publish(&ctx.session, event) {
                ctx.record
                    .lock()
                    .expect("record meta mutex poisoned")
                    .fanout_n = Some(reached);
            }
        }
        Ok(Outbound::Accepted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Kind, Token};
    use bytes::Bytes;
    use contracts::{CorrelationId, HookEvent, HookKind, SessionId};
    use std::time::Duration;

    fn ctx_with_event(session: &str, correlation: &str) -> Ctx {
        let event = HookEvent {
            session_id: SessionId(session.into()),
            correlation_id: CorrelationId(correlation.into()),
            ts: 0,
            kind: HookKind::Stop { turn_index: None },
        };
        Ctx {
            kind: Kind::Hook,
            session: SessionId(session.into()),
            correlation: CorrelationId(correlation.into()),
            token: Token::new("t"),
            body: Bytes::new(),
            event: Some(event),
            record: Default::default(),
        }
    }

    #[tokio::test]
    async fn fans_out_the_event_to_every_subscriber() {
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let session = SessionId("s".into());
        let mut a = subscriptions.subscribe(&session);
        let mut b = subscriptions.subscribe(&session);
        let fanout = FanOut::new(subscriptions);

        let out = fanout
            .handle(ctx_with_event("s", "c1"), Next::noop())
            .await
            .unwrap();

        assert_eq!(out, Outbound::Accepted);
        for rx in [&mut a, &mut b] {
            assert_eq!(
                rx.recv().await.unwrap().correlation_id,
                CorrelationId("c1".into())
            );
        }
    }

    #[tokio::test]
    async fn accepts_even_when_there_are_no_subscribers() {
        let fanout = FanOut::new(Arc::new(Subscriptions::with_capacity(8)));

        let out = fanout
            .handle(ctx_with_event("s", "c1"), Next::noop())
            .await
            .unwrap();

        assert_eq!(out, Outbound::Accepted);
    }

    #[tokio::test]
    async fn does_not_block_the_poster() {
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        // A subscriber that never reads must not stall the fan-out.
        let _rx = subscriptions.subscribe(&SessionId("s".into()));
        let fanout = FanOut::new(subscriptions);

        let out = tokio::time::timeout(
            Duration::from_millis(500),
            fanout.handle(ctx_with_event("s", "c1"), Next::noop()),
        )
        .await
        .expect("fan-out must not block")
        .unwrap();

        assert_eq!(out, Outbound::Accepted);
    }

    #[tokio::test]
    async fn is_terminal_and_never_calls_next() {
        let fanout = FanOut::new(Arc::new(Subscriptions::with_capacity(8)));
        let (next, called) = Next::spy();

        fanout
            .handle(ctx_with_event("s", "c1"), next)
            .await
            .unwrap();

        assert!(
            !*called.lock().unwrap(),
            "a terminal middleware never runs next"
        );
    }
}
