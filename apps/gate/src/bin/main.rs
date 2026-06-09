//! Gate binary entry point.

use tillerd_gate::service::Gate;

fn main() {
    service_host::run_blocking(Gate::from_env());
}
