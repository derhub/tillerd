#![forbid(unsafe_code)]

pub mod app;
pub mod boot;
pub mod context;
mod entities;
pub mod health;
mod infra;
mod middleware;
pub mod shared;
pub mod supervision;

pub use boot::{build_bus, Config};
pub use context::Ctx;
pub use health::{read_service_health, HealthSpec, ServiceHealth, ServiceState};
pub use shared::Bus;
