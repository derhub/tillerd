//! The gateway binary: run under the `service-host` lifecycle (path resolution,
//! manifest, signals, graceful shutdown).

use tillerd_mcp_gateway::service::GatewayService;

const SERVICE_NAME: &str = "tillerd-mcp-gateway";

fn main() {
    let dir = tillerd_paths::runtime_dir();
    let (_guard, root) =
        tillerd_paths::logging::init_file_tracing(SERVICE_NAME, env!("CARGO_PKG_VERSION"), &dir);
    let _root = root.entered();
    service_host::run_blocking(GatewayService::from_env());
}
