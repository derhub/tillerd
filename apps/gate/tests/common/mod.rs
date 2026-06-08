use std::net::SocketAddr;
use std::sync::Arc;

use athing_gate::endpoint::mcp;
use http::{HeaderName, HeaderValue};
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::ServerHandler;
use tokio::task::JoinHandle;

pub fn object_schema() -> Arc<serde_json::Map<String, serde_json::Value>> {
    Arc::new(
        serde_json::json!({ "type": "object" })
            .as_object()
            .unwrap()
            .clone(),
    )
}

/// Serve any handler over the gate's streamable-HTTP transport, ephemeral port.
pub async fn serve<H>(handler: H) -> (SocketAddr, JoinHandle<()>)
where
    H: ServerHandler + Clone + Send + Sync + 'static,
{
    let service = StreamableHttpService::new(
        move || Ok(handler.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );
    let app = axum::Router::new().nest_service("/mcp", service);
    let listener = mcp::bind(0).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (addr, server)
}

/// Client config: token in `Authorization`, session in `x-athing-session`.
pub fn client_config(
    addr: SocketAddr,
    session: Option<&str>,
    token: Option<&str>,
) -> StreamableHttpClientTransportConfig {
    // Client adds the `Bearer ` prefix, so pass the raw token.
    let mut config = StreamableHttpClientTransportConfig::with_uri(format!("http://{addr}/mcp"));
    config.auth_header = token.map(|t| t.to_string());
    if let Some(session) = session {
        config.custom_headers.insert(
            HeaderName::from_static("x-athing-session"),
            HeaderValue::from_str(session).unwrap(),
        );
    }
    config
}
