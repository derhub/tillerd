//! The MCP ingress face: an `rmcp` server, peer to the hook and tool faces.
//!
//! `rmcp` owns the protocol; gate owns only the bridge to an [`Inbound`]. Every
//! `rmcp` type is confined to this module, so an SDK upgrade touches one file.
//! Loopback HTTP (primary) carries the token/session in headers, the Unix socket
//! (secondary) in a handshake frame; both admit against the registry and route
//! through the shared `Auth` global.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bytes::Bytes;
use contracts::SessionId;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, ListToolsResult, PaginatedRequestParams,
    ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, ServiceExt};
use serde::Deserialize;
use tokio::net::{TcpListener, UnixListener, UnixStream};
use tokio::task::JoinHandle;

use crate::endpoint::read_frame;
use crate::registry::SessionRegistry;
use crate::router::{Inbound, Router};
use crate::{Kind, Outbound, Reject, Token};

/// The HTTP header the orchestrator injects to bind a connection to its session.
const SESSION_HEADER: &str = "x-athing-session";

/// Environment selector for the bound MCP transport; defaults to loopback HTTP.
const TRANSPORT_ENV: &str = "ATHING_GATE_MCP_TRANSPORT";

/// Environment override for the loopback HTTP port; `0` (the default) is ephemeral.
const PORT_ENV: &str = "ATHING_GATE_MCP_HTTP_PORT";

/// Which local transport the MCP face binds. The face binds exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// Loopback HTTP (streamable). The primary, client-facing transport.
    Http,
    /// A local Unix socket. The secondary, for same-host orchestrator clients.
    Socket,
}

/// The session and token an admitted connection authenticated with.
#[derive(Debug, Clone)]
struct Identity {
    session: SessionId,
    token: Token,
}

/// A socket admission handshake: the session and token, sent as the first frame.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Handshake {
    session: String,
    token: String,
}

/// The MCP `ServerHandler`: routes every call through the gate router. `bound` is
/// the socket's admission identity; HTTP reads it per request from headers.
#[derive(Clone)]
struct GateMcp {
    router: Arc<Router>,
    bound: Option<Identity>,
}

impl GateMcp {
    fn identity(&self, ctx: &RequestContext<RoleServer>) -> Result<Identity, McpError> {
        if let Some(bound) = &self.bound {
            return Ok(bound.clone());
        }
        let parts = ctx
            .extensions
            .get::<http::request::Parts>()
            .ok_or_else(|| McpError::invalid_request("missing request metadata", None))?;
        identity_from_headers(&parts.headers)
            .ok_or_else(|| McpError::invalid_request("missing session or token", None))
    }
}

impl ServerHandler for GateMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }

    async fn list_tools(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(Vec::new()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let Identity { session, token } = self.identity(&ctx)?;
        route_call(&self.router, session, token, request).await
    }
}

/// Bridge one MCP call to a `Kind::Mcp` inbound and map the routed outcome back.
/// Both transports converge here, so an identical call yields an identical outcome.
async fn route_call(
    router: &Router,
    session: SessionId,
    token: Token,
    request: CallToolRequestParams,
) -> Result<CallToolResult, McpError> {
    let body = serde_json::to_vec(&request)
        .map(Bytes::from)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let inbound = Inbound {
        kind: Kind::Mcp,
        session,
        correlation: None,
        token,
        body,
    };
    match router.handle(inbound).await {
        Ok(Outbound::Forward(body)) => Ok(CallToolResult::success(vec![Content::text(
            String::from_utf8_lossy(&body).into_owned(),
        )])),
        Ok(Outbound::Accepted) => Ok(CallToolResult::success(Vec::new())),
        Err(Reject::Unauthenticated) => Err(McpError::invalid_request("unauthenticated", None)),
        Err(other) => Err(McpError::invalid_params(other.to_string(), None)),
    }
}

/// Session + bearer token from request headers (tolerates a bare token).
fn identity_from_headers(headers: &http::HeaderMap) -> Option<Identity> {
    let session = headers.get(SESSION_HEADER)?.to_str().ok()?.to_string();
    let value = headers.get(http::header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ").unwrap_or(value).to_string();
    Some(Identity {
        session: SessionId(session),
        token: Token::new(token),
    })
}

/// Loopback HTTP app: the streamable MCP service at `/mcp` behind token admission.
pub fn http_app(router: Arc<Router>, registry: Arc<SessionRegistry>) -> axum::Router {
    let handler = GateMcp {
        router,
        bound: None,
    };
    let mcp = StreamableHttpService::new(
        move || Ok(handler.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );
    axum::Router::new()
        .nest_service("/mcp", mcp)
        .route_layer(axum::middleware::from_fn_with_state(registry, admit))
}

/// Refuse a request whose session token does not authenticate, before the MCP
/// service handles it; the routed call re-checks via the `Auth` global.
async fn admit(
    axum::extract::State(registry): axum::extract::State<Arc<SessionRegistry>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let admitted = identity_from_headers(req.headers())
        .is_some_and(|id| registry.verify(&id.session, &id.token).is_some());
    if admitted {
        next.run(req).await
    } else {
        (axum::http::StatusCode::UNAUTHORIZED, "unauthorized").into_response()
    }
}

/// Bind the loopback HTTP listener on `127.0.0.1:port` (`port = 0` is ephemeral).
pub async fn bind(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await
}

/// Bind and serve the Unix-socket transport until the task is aborted.
pub fn serve_socket(
    socket_path: PathBuf,
    router: Arc<Router>,
    registry: Arc<SessionRegistry>,
) -> std::io::Result<JoinHandle<()>> {
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    Ok(tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(serve_socket_conn(stream, router.clone(), registry.clone()));
        }
    }))
}

async fn serve_socket_conn(
    mut stream: UnixStream,
    router: Arc<Router>,
    registry: Arc<SessionRegistry>,
) {
    let Ok(Some(frame)) = read_frame(&mut stream).await else {
        return;
    };
    let Ok(handshake) = serde_json::from_slice::<Handshake>(&frame) else {
        return;
    };
    let identity = Identity {
        session: SessionId(handshake.session),
        token: Token::new(handshake.token),
    };
    if registry
        .verify(&identity.session, &identity.token)
        .is_none()
    {
        return;
    }
    let handler = GateMcp {
        router,
        bound: Some(identity),
    };
    if let Ok(service) = handler.serve(stream).await {
        let _ = service.waiting().await;
    }
}

/// The published MCP endpoint for a bound loopback address: the streamable HTTP URL.
pub fn mcp_url(addr: SocketAddr) -> String {
    format!("http://{addr}/mcp")
}

/// Publish the bound endpoint to `path`; the per-session token is never written.
pub fn write_mcp_url(path: &Path, addr: SocketAddr) -> std::io::Result<()> {
    std::fs::write(path, mcp_url(addr))
}

/// Resolve the bound transport from the environment; defaults to loopback HTTP.
pub fn transport_from_env() -> Transport {
    match std::env::var(TRANSPORT_ENV).ok().as_deref() {
        Some("socket") => Transport::Socket,
        _ => Transport::Http,
    }
}

/// Resolve the loopback HTTP port from the environment; defaults to ephemeral.
pub fn port_from_env() -> u16 {
    std::env::var(PORT_ENV)
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::auth::Auth;
    use crate::middleware::passthrough::PassThrough;
    use crate::middleware::Middleware;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use rmcp::model::CallToolRequestParams;
    use std::collections::HashMap;
    use tower::ServiceExt as _;

    fn registry_with(session: &str, token: &str) -> Arc<SessionRegistry> {
        let registry = Arc::new(SessionRegistry::new());
        registry.register(SessionId(session.into()), &Token::new(token));
        registry
    }

    fn mcp_router(registry: Arc<SessionRegistry>) -> Arc<Router> {
        let globals = vec![Arc::new(Auth::new(registry)) as Arc<dyn Middleware>];
        let routes = HashMap::from([(Kind::Mcp, Arc::new(PassThrough) as Arc<dyn Middleware>)]);
        Arc::new(Router::new(globals, routes))
    }

    fn call(name: &str) -> CallToolRequestParams {
        CallToolRequestParams::new(name.to_string())
            .with_arguments(serde_json::json!({ "k": "v" }).as_object().unwrap().clone())
    }

    fn temp_sock(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        PathBuf::from(format!(
            "/tmp/gate-mcp-{tag}-{}-{nanos}.sock",
            std::process::id()
        ))
    }

    /// A client connected to a `GateMcp` served over an in-memory duplex.
    async fn connect_in_memory(
        router: Arc<Router>,
        session: &str,
        token: &str,
    ) -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
        let (server_io, client_io) = tokio::io::duplex(8192);
        let handler = GateMcp {
            router,
            bound: Some(Identity {
                session: SessionId(session.into()),
                token: Token::new(token),
            }),
        };
        tokio::spawn(async move {
            if let Ok(service) = handler.serve(server_io).await {
                let _ = service.waiting().await;
            }
        });
        ().serve(client_io).await.expect("client initializes")
    }

    #[tokio::test]
    async fn a_compliant_client_completes_the_initialize_handshake() {
        // A successful `serve` is a completed handshake; rmcp owns version negotiation.
        let client =
            connect_in_memory(mcp_router(registry_with("s", "secret")), "s", "secret").await;

        assert!(
            client.peer_info().is_some(),
            "the negotiated server info is available after a completed handshake"
        );

        let _ = client.cancel().await;
    }

    #[tokio::test]
    async fn a_tool_listing_returns_an_empty_set() {
        let client =
            connect_in_memory(mcp_router(registry_with("s", "secret")), "s", "secret").await;

        let tools = client.list_tools(Default::default()).await.unwrap();

        assert!(tools.tools.is_empty(), "the routing layer carries no tools");

        let _ = client.cancel().await;
    }

    async fn status_for(registry: Arc<SessionRegistry>, headers: &[(&str, &str)]) -> StatusCode {
        let app = http_app(mcp_router(registry.clone()), registry);
        let mut builder = Request::builder().method("POST").uri("/mcp");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let req = builder.body(Body::from("{}")).unwrap();
        app.oneshot(req).await.unwrap().status()
    }

    #[tokio::test]
    async fn a_connection_with_no_token_is_refused_at_admission() {
        let status = status_for(registry_with("s", "secret"), &[]).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn a_wrong_token_is_refused_at_admission() {
        let status = status_for(
            registry_with("s", "secret"),
            &[("x-athing-session", "s"), ("authorization", "Bearer wrong")],
        )
        .await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn a_valid_token_is_admitted_past_the_gate() {
        let status = status_for(
            registry_with("s", "secret"),
            &[
                ("x-athing-session", "s"),
                ("authorization", "Bearer secret"),
            ],
        )
        .await;

        assert_ne!(
            status,
            StatusCode::UNAUTHORIZED,
            "a valid session token is admitted to the protocol loop"
        );
    }

    #[tokio::test]
    async fn a_wrong_token_is_rejected_before_routing() {
        let result = route_call(
            &mcp_router(registry_with("s", "secret")),
            SessionId("s".into()),
            Token::new("wrong"),
            call("anything"),
        )
        .await;

        let err = result.unwrap_err();
        assert_eq!(err.message.as_ref(), "unauthenticated");
    }

    #[tokio::test]
    async fn the_http_transport_binds_loopback_only() {
        let listener = bind(0).await.unwrap();

        assert!(
            listener.local_addr().unwrap().ip().is_loopback(),
            "the mcp http face binds 127.0.0.1 only"
        );
    }

    #[tokio::test]
    async fn the_same_authenticated_call_yields_identical_outcomes_over_either_transport() {
        let registry = registry_with("s", "secret");
        let router = mcp_router(registry.clone());

        let core = route_call(
            &router,
            SessionId("s".into()),
            Token::new("secret"),
            call("echo"),
        )
        .await
        .unwrap();

        let sock = temp_sock("xport");
        let handle = serve_socket(sock.clone(), router, registry).unwrap();
        let mut stream = UnixStream::connect(&sock).await.unwrap();
        let handshake =
            serde_json::to_vec(&serde_json::json!({ "session": "s", "token": "secret" })).unwrap();
        crate::endpoint::write_frame(&mut stream, &handshake)
            .await
            .unwrap();
        let client = ().serve(stream).await.unwrap();

        let over_socket = client.call_tool(call("echo")).await.unwrap();

        assert_eq!(
            serde_json::to_value(&over_socket).unwrap(),
            serde_json::to_value(&core).unwrap(),
            "the socket transport yields the same outcome as the core bridge"
        );

        let _ = client.cancel().await;
        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }

    #[test]
    fn mcp_url_points_at_the_streamable_endpoint() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        assert_eq!(mcp_url(addr), "http://127.0.0.1:8080/mcp");
    }

    #[test]
    fn write_mcp_url_publishes_the_endpoint_and_never_a_token() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gate-mcp.url");
        let addr: SocketAddr = "127.0.0.1:9091".parse().unwrap();

        write_mcp_url(&path, addr).unwrap();

        let published = std::fs::read_to_string(&path).unwrap();
        assert_eq!(published, "http://127.0.0.1:9091/mcp");
        assert!(
            !published.to_lowercase().contains("token")
                && !published.contains("Bearer")
                && !published.contains("secret"),
            "the discovery entry carries the endpoint only, no secret"
        );
    }

    #[test]
    fn transport_defaults_to_http() {
        // Env-driven; assert the default arm directly to avoid global env races.
        assert_eq!(transport_from_env(), Transport::Http);
    }

    #[tokio::test]
    async fn route_call_forwards_an_authenticated_call_as_a_success_result() {
        let result = route_call(
            &mcp_router(registry_with("s", "secret")),
            SessionId("s".into()),
            Token::new("secret"),
            call("echo"),
        )
        .await
        .unwrap();

        assert_eq!(result.is_error, Some(false));
        assert!(
            !result.content.is_empty(),
            "the forwarded body is returned as content"
        );
    }
}
