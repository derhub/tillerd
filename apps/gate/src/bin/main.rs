//! Gate binary entry point.

use tillerd_gate::service::Gate;

const SERVICE_NAME: &str = "tillerd-gate";

fn main() {
    let dir = tillerd_paths::runtime_dir();
    let (_guard, root) =
        tillerd_paths::logging::init_file_tracing(SERVICE_NAME, env!("CARGO_PKG_VERSION"), &dir);
    let _root = root.entered();
    service_host::run_blocking(Gate::from_env());
}
