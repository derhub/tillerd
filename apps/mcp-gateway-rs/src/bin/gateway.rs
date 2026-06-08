//! The gateway binary: run under the `service-host` lifecycle (path resolution,
//! manifest, signals, liveness probe, graceful shutdown).

use athing_mcp_gateway::service::GatewayService;

fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    if let Err(error) = rt.block_on(service_host::host::run(GatewayService::from_env())) {
        eprintln!("gateway serve error: {error}");
        std::process::exit(1);
    }
}
