//! Auth: constant-time token verification. Rejects if unauthenticated.

use std::sync::Arc;

use async_trait::async_trait;

use crate::middleware::{Middleware, Next};
use crate::registry::SessionRegistry;
use crate::{Ctx, Flow, Reject};

/// Rejects inbounds whose bearer token does not match the session's registry entry.
pub struct Auth {
    registry: Arc<SessionRegistry>,
}

impl Auth {
    /// Build an auth layer backed by the given session registry.
    pub fn new(registry: Arc<SessionRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Middleware for Auth {
    async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
        if self.registry.verify(&ctx.session, &ctx.token).is_some() {
            next.run(ctx).await
        } else {
            Err(Reject::Unauthenticated)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Kind, Outbound, Token};
    use bytes::Bytes;
    use contracts::{CorrelationId, SessionId};

    fn ctx(session: &str, token: &str) -> Ctx {
        Ctx {
            kind: Kind::Hook,
            session: SessionId(session.into()),
            correlation: CorrelationId("c".into()),
            token: Token::new(token),
            body: Bytes::new(),
            event: None,
            record: Default::default(),
        }
    }

    fn registry_with(session: &str, token: &str) -> Arc<SessionRegistry> {
        let registry = Arc::new(SessionRegistry::new());
        registry.register(SessionId(session.into()), &Token::new(token));
        registry
    }

    #[tokio::test]
    async fn passes_and_continues_when_token_matches() {
        let auth = Auth::new(registry_with("s1", "secret"));
        let (next, called) = Next::spy();

        let out = auth.handle(ctx("s1", "secret"), next).await.unwrap();

        assert_eq!(out, Outbound::Accepted);
        assert!(*called.lock().unwrap(), "auth continues to next on a match");
    }

    #[tokio::test]
    async fn rejects_when_session_not_registered() {
        let auth = Auth::new(Arc::new(SessionRegistry::new()));
        let (next, called) = Next::spy();

        let err = auth.handle(ctx("ghost", "secret"), next).await.unwrap_err();

        assert_eq!(err, Reject::Unauthenticated);
        assert!(!*called.lock().unwrap(), "auth never runs next on reject");
    }

    #[tokio::test]
    async fn rejects_and_short_circuits_when_token_mismatches() {
        let auth = Auth::new(registry_with("s1", "secret"));
        let (next, called) = Next::spy();

        let err = auth.handle(ctx("s1", "wrong"), next).await.unwrap_err();

        assert_eq!(err, Reject::Unauthenticated);
        assert!(
            !*called.lock().unwrap(),
            "a mismatch short-circuits without running next"
        );
    }
}
