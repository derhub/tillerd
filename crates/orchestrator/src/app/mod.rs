//! Application use-case layer: host-agnostic cross-aggregate coordination.
//!
//! Use cases sequence work across per-entity stores (and narrow infra ports) that no single store
//! may own and that hosts must not assemble themselves. Hosts (tauri, future server) delegate here.

pub mod command;
pub mod notification;
pub mod project;
pub mod session;
pub mod settings;
pub mod surface;
pub mod template;
pub mod workspace;
