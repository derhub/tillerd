#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Uniform run-me lifecycle for long-lived tools.
//!
//! Every long-lived tool starts through one host entry point — [`host::run`] —
//! which resolves the tool's resource paths, writes its manifest, installs
//! signal handlers, exposes an unauthenticated liveness probe, and runs the
//! tool's serve behavior, then performs an escalating graceful-then-forced
//! shutdown that leaves no orphaned children. A tool supplies only its identity
//! and its serve behavior via the [`Service`] trait; it never reimplements the
//! plumbing.
//!
//! This crate is lifecycle and filesystem only. It carries no wire types and no
//! transport: each adopting tool keeps its own protocol.

pub mod host;
pub mod manifest;
pub mod paths;
pub mod probe;
pub mod shutdown;
pub mod signals;

pub use host::{run, Service, ServiceConfig};
pub use manifest::{Manifest, ManifestData};
pub use paths::Paths;
pub use shutdown::{ChildRegistry, DEFAULT_GRACE_PERIOD};
