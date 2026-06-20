//! Application use-case layer: host-agnostic cross-aggregate coordination.
//!
//! Use cases sequence work across per-entity stores (and narrow infra ports) that no single store
//! may own and that hosts must not assemble themselves. Hosts (tauri, future server) delegate here.

pub mod session;

pub use session::{create_session, open_session, SessionActivator};
