//! MCP ingress: rmcp types confined to this module (SDK upgrades touch one file).
//! Served as the `Mcp` route of the gate's single socket: the route preamble admits
//! the connection, then the stream upgrades to the MCP protocol.

use std::sync::Arc;

use bytes::Bytes;
use contracts::SessionId;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, ServiceExt};
use tokio::net::UnixStream;

use crate::router::{Inbound, Router};
use crate::{Kind, Outbound, Reject, Token};

/// The session and token an admitted connection authenticated with via its preamble.
#[derive(Debug, Clone)]
struct Identity {
    session: SessionId,
    token: Token,
}

/// The MCP `ServerHandler`: routes every call through the gate router. `identity`
/// is the connection's preamble-admitted session and token.
#[derive(Clone)]
struct GateMcp {
    router: Arc<Router>,
    identity: Identity,
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
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let Identity { session, token } = self.identity.clone();
        route_call(&self.router, session, token, request).await
    }
}

/// Serve one MCP connection whose route preamble already admitted `session`/`token`:
/// upgrade the stream to the MCP protocol and bridge calls to `Kind::Mcp` inbounds.
pub async fn serve_conn(stream: UnixStream, router: Arc<Router>, session: SessionId, token: Token) {
    let handler = GateMcp {
        router,
        identity: Identity { session, token },
    };
    if let Ok(service) = handler.serve(stream).await {
        let _ = service.waiting().await;
    }
}

/// Bridge one MCP call to a `Kind::Mcp` inbound and map the routed outcome back.
async fn route_call(
    router: &Router,
    session: SessionId,
    token: Token,
    request: CallToolRequestParams,
) -> Result<CallToolResponse, McpError> {
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
        Ok(Outbound::Forward(body)) => Ok(CallToolResult::success(vec![ContentBlock::text(
            String::from_utf8_lossy(&body).into_owned(),
        )])
        .into()),
        Ok(Outbound::Accepted) => Ok(CallToolResult::success(Vec::new()).into()),
        Err(Reject::Unauthenticated) => Err(McpError::invalid_request("unauthenticated", None)),
        Err(other) => Err(McpError::invalid_params(other.to_string(), None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::auth::Auth;
    use crate::middleware::passthrough::PassThrough;
    use crate::middleware::Middleware;
    use crate::registry::SessionRegistry;
    use rmcp::model::CallToolRequestParams;
    use std::collections::HashMap;
    use std::path::PathBuf;

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
            identity: Identity {
                session: SessionId(session.into()),
                token: Token::new(token),
            },
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
    async fn serve_conn_yields_the_same_outcome_as_the_core_bridge() {
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
        let _ = std::fs::remove_file(&sock);
        let listener = tokio::net::UnixListener::bind(&sock).unwrap();
        let serve_router = router.clone();
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            // The demux admits via the preamble; serve_conn receives the upgraded stream.
            serve_conn(
                stream,
                serve_router,
                SessionId("s".into()),
                Token::new("secret"),
            )
            .await;
        });
        let stream = UnixStream::connect(&sock).await.unwrap();
        let client = ().serve(stream).await.unwrap();

        let over_socket = client.call_tool(call("echo")).await.unwrap();

        assert_eq!(
            serde_json::to_value(&over_socket).unwrap(),
            serde_json::to_value(&core).unwrap(),
            "the socket route yields the same outcome as the core bridge"
        );

        let _ = client.cancel().await;
        handle.abort();
        let _ = std::fs::remove_file(&sock);
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
