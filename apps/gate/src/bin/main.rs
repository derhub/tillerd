//! The gate binary: build the gate from the environment and run it under the
//! `service-host` lifecycle (path resolution, manifest, signals, probe, shutdown).

use athing_gate::service::Gate;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    if let Err(error) = rt.block_on(service_host::host::run(Gate::from_env())) {
        eprintln!("gate serve error: {error}");
        std::process::exit(1);
    }
}
