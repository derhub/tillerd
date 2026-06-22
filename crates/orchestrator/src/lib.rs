#![forbid(unsafe_code)]

pub mod app;
pub mod boot;
pub mod context;
pub mod entities;
pub mod health;
pub mod infra;
pub mod shared;
pub mod supervision;

pub use boot::{build_bus, Config};
pub use context::Ctx;
pub use health::{read_service_health, HealthSpec, ServiceHealth, ServiceState};
pub use shared::Bus;
