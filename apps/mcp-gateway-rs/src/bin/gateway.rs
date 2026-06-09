//! The gateway binary: run under the `service-host` lifecycle (path resolution,
//! manifest, signals, graceful shutdown).

use athing_mcp_gateway::service::GatewayService;

fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    service_host::run_blocking(GatewayService::from_env());
}
