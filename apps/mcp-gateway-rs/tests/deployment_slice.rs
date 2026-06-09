//! Deployment slice: gateway only (no daemon, gate, or memorya).
//! the binary uses. It proves the slice boots and is ready to serve. For a
//! slice that also exercises the HTTP face with a real MCP client, see
//! `integration.rs`.

use athing_mcp_gateway::{build, McpConfig};

/// An empty-config gateway builds and exposes a live supervisor with no tools.
/// This is the minimal gateway-only slice: no backends, no daemon, no gate.
#[tokio::test]
async fn gateway_only_slice_boots_with_empty_config() {
    let config = McpConfig::default();
    let gw = build(config).await.expect("gateway-only slice builds");

    let tools = gw.supervisor().registry().all_tools();
    assert!(
        tools.is_empty(),
        "empty config → zero tools; got: {tools:?}"
    );

    gw.supervisor().shutdown().await;
}
