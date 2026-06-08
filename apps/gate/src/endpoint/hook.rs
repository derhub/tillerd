//! The hook ingress face: a loopback HTTP endpoint the agent's hook posts to.
//!
//! It reads the per-session token from the `Authorization` header, caps the body
//! (the OOM guard), acknowledges with `200` before the router normalizes and fans
//! out (fire-and-forget), then drives the router's `Hook` route in the background.
//! The bound address is published to `<base>/gate.url` so the orchestrator can
//! inject it into the agent's hook configuration.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Path as RoutePath, State};
use axum::http::{header, HeaderMap, StatusCode};
use bytes::Bytes;
use contracts::SessionId;
use tokio::net::TcpListener;

use crate::router::{Inbound, Router};
use crate::{Kind, Token};

/// The hook body cap when the override is unset: 1 MiB.
const DEFAULT_MAX_BODY: usize = 1024 * 1024;

/// Environment override for the hook body cap (the OOM guard).
const MAX_BODY_ENV: &str = "ATHING_GATE_HOOK_MAX_BODY";

/// Environment override for the loopback port; `0` (the default) binds ephemeral.
const PORT_ENV: &str = "ATHING_GATE_PORT";

/// Build the loopback HTTP app: one body-capped hook route driving the router.
pub fn app(router: Arc<Router>, max_body: usize) -> axum::Router {
    axum::Router::new()
        .route("/hook/{session}", axum::routing::post(handle_hook))
        .layer(DefaultBodyLimit::max(max_body))
        .with_state(router)
}

async fn handle_hook(
    State(router): State<Arc<Router>>,
    RoutePath(session): RoutePath<String>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let token = Token::new(bearer_token(&headers).unwrap_or_default());
    let inbound = Inbound {
        kind: Kind::Hook,
        session: SessionId(session),
        correlation: None,
        token,
        body,
    };
    // Fire-and-forget: acknowledge before the router normalizes and fans out so a
    // slow or absent subscriber never stalls the agent's hook.
    tokio::spawn(async move {
        let _ = router.handle(inbound).await;
    });
    StatusCode::OK
}

/// Read the token from the `Authorization` header, tolerating a bare token
/// without the `Bearer ` scheme.
fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    Some(value.strip_prefix("Bearer ").unwrap_or(value).to_string())
}

/// Bind the loopback listener on `127.0.0.1:port` (`port = 0` is ephemeral).
pub async fn bind(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await
}

/// The published gate URL for a bound loopback address.
pub fn gate_url(addr: SocketAddr) -> String {
    format!("http://{addr}")
}

/// Write the published gate URL to `path` for the orchestrator to read.
pub fn write_gate_url(path: &Path, addr: SocketAddr) -> std::io::Result<()> {
    std::fs::write(path, gate_url(addr))
}

/// Resolve the hook body cap from the environment.
pub fn max_body_from_env() -> usize {
    resolve_max_body(std::env::var(MAX_BODY_ENV).ok().as_deref())
}

/// Resolve the loopback port from the environment.
pub fn port_from_env() -> u16 {
    resolve_port(std::env::var(PORT_ENV).ok().as_deref())
}

fn resolve_max_body(raw: Option<&str>) -> usize {
    raw.and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_MAX_BODY)
}

fn resolve_port(raw: Option<&str>) -> u16 {
    raw.and_then(|v| v.parse::<u16>().ok()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_adapter::V1Adapter;
    use crate::middleware::auth::Auth;
    use crate::middleware::fanout::FanOut;
    use crate::middleware::normalize::Normalize;
    use crate::middleware::{Middleware, Next};
    use crate::registry::SessionRegistry;
    use crate::subscription::Subscriptions;
    use crate::{Ctx, Flow};
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::Request;
    use contracts::HookKind;
    use std::collections::HashMap;
    use std::time::Duration;
    use tower::ServiceExt;

    fn post(uri: &str, token: Option<&str>, body: &[u8]) -> Request<Body> {
        let mut builder = Request::builder().method("POST").uri(uri);
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        builder.body(Body::from(body.to_vec())).unwrap()
    }

    fn registry_with(session: &str, token: &str) -> Arc<SessionRegistry> {
        let registry = Arc::new(SessionRegistry::new());
        registry.register(SessionId(session.into()), &Token::new(token));
        registry
    }

    fn hook_router(
        registry: Arc<SessionRegistry>,
        subscriptions: Arc<Subscriptions>,
    ) -> Arc<Router> {
        let globals = vec![Arc::new(Auth::new(registry)) as Arc<dyn Middleware>];
        let route = crate::middleware::seq(vec![
            Arc::new(Normalize::new(Arc::new(V1Adapter))),
            Arc::new(FanOut::new(subscriptions)),
        ]);
        Arc::new(Router::new(globals, HashMap::from([(Kind::Hook, route)])))
    }

    #[test]
    fn body_cap_defaults_to_one_mebibyte_when_unset() {
        assert_eq!(resolve_max_body(None), 1024 * 1024);
    }

    #[test]
    fn body_cap_uses_a_valid_override() {
        assert_eq!(resolve_max_body(Some("4096")), 4096);
    }

    #[test]
    fn body_cap_falls_back_when_the_override_is_not_a_positive_number() {
        assert_eq!(resolve_max_body(Some("0")), 1024 * 1024);
        assert_eq!(resolve_max_body(Some("lots")), 1024 * 1024);
    }

    #[test]
    fn port_defaults_to_ephemeral_when_unset_or_invalid() {
        assert_eq!(resolve_port(None), 0);
        assert_eq!(resolve_port(Some("not-a-port")), 0);
        assert_eq!(resolve_port(Some("8081")), 8081);
    }

    #[test]
    fn gate_url_formats_a_loopback_address() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        assert_eq!(gate_url(addr), "http://127.0.0.1:8080");
    }

    #[test]
    fn write_gate_url_publishes_the_url_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gate.url");
        let addr: SocketAddr = "127.0.0.1:9090".parse().unwrap();

        write_gate_url(&path, addr).unwrap();

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "http://127.0.0.1:9090"
        );
    }

    #[tokio::test]
    async fn bind_listens_only_on_loopback() {
        let listener = bind(0).await.unwrap();
        assert!(
            listener.local_addr().unwrap().ip().is_loopback(),
            "the hook face binds 127.0.0.1 only"
        );
    }

    #[tokio::test]
    async fn rejects_a_body_over_the_cap() {
        let router = Arc::new(Router::new(vec![], HashMap::new()));
        let app = app(router, 8);

        let response = app
            .oneshot(post("/hook/s", Some("t"), &[b'x'; 64]))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    struct Pending;

    #[async_trait]
    impl Middleware for Pending {
        async fn handle(&self, _ctx: Ctx, _next: Next<'_>) -> Flow {
            std::future::pending().await
        }
    }

    #[tokio::test]
    async fn acknowledges_before_the_route_runs_to_completion() {
        // The route never completes; a 200 still proves the ack precedes fan-out.
        let router = Arc::new(Router::new(
            vec![],
            HashMap::from([(Kind::Hook, Arc::new(Pending) as Arc<dyn Middleware>)]),
        ));
        let app = app(router, 1 << 20);

        let response = app
            .oneshot(post("/hook/s", Some("t"), b"{}"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn threads_the_header_token_so_a_valid_hook_fans_out() {
        let registry = registry_with("s", "secret");
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let mut rx = subscriptions.subscribe(&SessionId("s".into()));
        let router = hook_router(registry, subscriptions.clone());
        let body =
            br#"{"hook_event_name":"Stop","session_id":"agent","timestamp_ms":5,"turn_index":2}"#;

        let response = app(router, 1 << 20)
            .oneshot(post("/hook/s", Some("secret"), body))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("the authenticated hook fans out")
            .unwrap();
        assert_eq!(
            event.kind,
            HookKind::Stop {
                turn_index: Some(2)
            }
        );
    }

    #[tokio::test]
    async fn a_wrong_header_token_never_fans_out() {
        let registry = registry_with("s", "secret");
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let mut rx = subscriptions.subscribe(&SessionId("s".into()));
        // Retained so the broadcast sender outlives the dropped router; otherwise
        // recv would observe a channel close rather than the absence of an event.
        let router = hook_router(registry, subscriptions.clone());
        let body = br#"{"hook_event_name":"Stop","session_id":"agent","timestamp_ms":5}"#;

        let response = app(router, 1 << 20)
            .oneshot(post("/hook/s", Some("wrong"), body))
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "the ack is unconditional"
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(200), rx.recv())
                .await
                .is_err(),
            "an unauthenticated hook never reaches fan-out"
        );
    }
}
