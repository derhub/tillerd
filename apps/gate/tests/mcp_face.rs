//! End-to-end tests for the gate MCP face: a real bound HTTP transport driven by
//! an `rmcp` streamable-HTTP client (handshake, list, call, admission), plus a
//! mock service proving a tool surfaces over the transport.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use athing_gate::endpoint::mcp;
use athing_gate::middleware::auth::Auth;
use athing_gate::middleware::passthrough::PassThrough;
use athing_gate::middleware::Middleware;
use athing_gate::registry::SessionRegistry;
use athing_gate::router::Router;
use athing_gate::{Kind, Token};
use contracts::SessionId;
use rmcp::model::{
    CallToolRequestParams, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo,
    Tool,
};
use rmcp::service::RequestContext;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, ServiceExt};
use tokio::task::JoinHandle;

mod common;

/// Router with the production `Auth` global and the `Kind::Mcp` route.
fn router(registry: Arc<SessionRegistry>) -> Arc<Router> {
    let globals: Vec<Arc<dyn Middleware>> = vec![Arc::new(Auth::new(registry))];
    let routes = HashMap::from([(Kind::Mcp, Arc::new(PassThrough) as Arc<dyn Middleware>)]);
    Arc::new(Router::new(globals, routes))
}

/// Bind and serve the real HTTP face; registry holds one session `s`/`secret`.
async fn serve_face() -> (SocketAddr, JoinHandle<()>) {
    let registry = Arc::new(SessionRegistry::new());
    registry.register(SessionId("s".into()), &Token::new("secret"));
    let listener = mcp::bind(0).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = mcp::http_app(router(registry.clone()), registry);
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (addr, server)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_authenticated_client_handshakes_lists_empty_tools_and_routes_a_call() {
    let (addr, server) = serve_face().await;

    let transport = StreamableHttpClientTransport::from_config(common::client_config(
        addr,
        Some("s"),
        Some("secret"),
    ));
    let client = ()
        .serve(transport)
        .await
        .expect("a valid session token completes the initialize handshake");

    assert!(
        client.peer_info().is_some(),
        "the handshake negotiated server info"
    );

    let tools = client.list_tools(Default::default()).await.unwrap();
    assert!(tools.tools.is_empty(), "the routing layer carries no tools");

    let result = client
        .call_tool(CallToolRequestParams::new("echo".to_string()))
        .await
        .unwrap();
    assert_eq!(
        result.is_error,
        Some(false),
        "an authenticated call is routed and forwarded as a success result"
    );
    assert!(
        !result.content.is_empty(),
        "the forwarded body is returned as content"
    );

    let _ = client.cancel().await;
    server.abort();
}

/// A stand-in handler exposing one tool, proving the listing path carries a tool.
#[derive(Clone)]
struct MockToolService;

impl ServerHandler for MockToolService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }

    async fn list_tools(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(vec![Tool::new(
            "ping",
            "a mock tool exposed for the listing test",
            common::object_schema(),
        )]))
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_client_lists_a_tool_exposed_over_the_mcp_transport() {
    let (addr, server) = common::serve(MockToolService).await;

    let transport =
        StreamableHttpClientTransport::from_config(common::client_config(addr, None, None));
    let client = ().serve(transport).await.expect("the client completes the initialize handshake");

    let tools = client.list_tools(Default::default()).await.unwrap();

    let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    assert_eq!(
        names,
        vec!["ping"],
        "the tool the service exposes is listed by name over the mcp transport"
    );

    let _ = client.cancel().await;
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_client_without_a_token_is_refused_before_the_protocol_loop() {
    let (addr, server) = serve_face().await;

    let transport =
        StreamableHttpClientTransport::from_config(common::client_config(addr, None, None));
    let outcome = ().serve(transport).await;

    assert!(
        outcome.is_err(),
        "initialize is refused at admission without a valid token"
    );

    server.abort();
}
