//! Standalone gateway daemon entry point.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_writer(std::io::stderr).init();
    athing_mcp_gateway::daemon::run().await
}
