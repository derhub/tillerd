#![forbid(unsafe_code)]

pub mod boot;
pub mod entities;
pub mod error;
pub mod health;
pub mod infra;
pub mod launch;
pub mod persistence;
pub mod supervision;
pub mod surface;

pub use boot::{boot, EventSink, Orchestrator, Status};
pub use error::{OrchestratorError, Result};
pub use health::{read_service_health, HealthSpec, ServiceHealth, ServiceState};
