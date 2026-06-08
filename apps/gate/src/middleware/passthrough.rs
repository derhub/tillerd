//! The pass-through terminal: forwards the inbound body unchanged. The gate
//! observes tool-route traffic without rewriting it (v1 allow-all).

use async_trait::async_trait;

use crate::middleware::{Middleware, Next};
use crate::{Ctx, Flow, Outbound};

/// Forwards `ctx.body` verbatim. Terminal: it never calls `next`.
pub struct PassThrough;

#[async_trait]
impl Middleware for PassThrough {
    async fn handle(&self, ctx: Ctx, _next: Next<'_>) -> Flow {
        Ok(Outbound::Forward(ctx.body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Kind, Token};
    use bytes::Bytes;
    use contracts::{CorrelationId, SessionId};

    fn ctx(body: &[u8]) -> Ctx {
        Ctx {
            kind: Kind::ToolCall,
            session: SessionId("s".into()),
            correlation: CorrelationId("c".into()),
            token: Token::new("t"),
            body: Bytes::copy_from_slice(body),
            event: None,
            record: Default::default(),
        }
    }

    #[tokio::test]
    async fn forwards_the_body_unchanged() {
        let out = PassThrough
            .handle(ctx(b"tool-payload"), Next::noop())
            .await
            .unwrap();

        assert_eq!(out, Outbound::Forward(Bytes::from_static(b"tool-payload")));
    }

    #[tokio::test]
    async fn is_terminal_and_never_calls_next() {
        let (next, called) = Next::spy();

        PassThrough.handle(ctx(b"x"), next).await.unwrap();

        assert!(
            !*called.lock().unwrap(),
            "a terminal middleware never runs next"
        );
    }
}
