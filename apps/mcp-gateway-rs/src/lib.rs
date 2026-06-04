#![deny(unsafe_code)]
//! Lightweight local-first MCP gateway: aggregates many MCP servers behind one
//! MCP face. Pure core (`config`, `router`, `registry`) carries no I/O;
//! `backend`/`supervisor` own all process and network side effects;
//! `handler` is the MCP server face.

pub mod backend;
pub mod config;
pub mod daemon;
pub mod front;
pub mod handler;
pub mod manifest;
pub mod registry;
pub mod router;
pub mod supervisor;
pub mod transport;

use std::sync::Arc;

pub use config::{BackendSpec, McpConfig};
pub use handler::Gateway;


pub async fn build(config: McpConfig) -> anyhow::Result<Gateway> {
    config.validate()?;
    let registry = registry::Registry::default();
    let front = front::FrontPeer::default();
    let (supervisor, refresh_rx) =
        supervisor::Supervisor::new(config, registry, front.clone());
    let supervisor = Arc::new(supervisor);

    supervisor.run_refresh_loop(refresh_rx);
    supervisor.start().await;

    Ok(Gateway::new(supervisor, front))
}

// Single source of truth for schema.json; the golden test guards drift.
pub fn config_schema_json() -> String {
    let schema = schemars::schema_for!(config::McpConfig);
    serde_json::to_string_pretty(&schema).expect("schema serializes")
}
