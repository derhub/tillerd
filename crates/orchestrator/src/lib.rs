#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! The tillerd orchestrator: a runtime-agnostic backend a host embeds in-process.
//!
//! The orchestrator owns the backend (ADR-0022). It boots through a defined
//! lifecycle — open the durable product store, adopt-or-spawn and health-check
//! the shared services, then reach an observable `ready` state — and exposes a
//! transport-agnostic API: request/response methods plus outbound lifecycle
//! events delivered through an [`EventSink`](boot::EventSink) the host
//! implements and binds to its own transport. It depends on no host runtime or
//! UI toolkit, so the same orchestrator can be embedded by different hosts.

pub mod boot;
pub mod error;
pub mod persistence;
pub mod supervision;

pub use boot::{boot, EventSink, Orchestrator, Status};
pub use error::{OrchestratorError, Result};
