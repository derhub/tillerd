#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Tool lifecycle host: single entry point ([`host::run`] / [`run_blocking`]), tool supplies only serve behavior.
//! Lifecycle and filesystem only; no wire types or transport (tools own protocol).

pub mod host;
pub mod manifest;
pub mod paths;
pub mod shutdown;
pub mod signals;

pub use host::{
    run, run_blocking, Drain, HealthReport, HealthStatus, Ready, ServeContext, Service,
    ServiceConfig,
};
pub use manifest::{Manifest, ManifestData, ServiceStatus};
pub use paths::Paths;
pub use shutdown::{ChildRegistry, DEFAULT_GRACE_PERIOD};
