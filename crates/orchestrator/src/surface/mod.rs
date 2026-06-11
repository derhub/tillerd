//! Terminal surface layer: async daemon-socket transport and session management.
//!
//! The surface module bridges the orchestrator to the PTY daemon via a Unix domain
//! socket using the `daemon-pty-client` codec for framing and typed frame decoding.

pub mod api;
pub mod runtime;
pub mod transport;

pub use api::SurfaceApi;
pub use runtime::{SurfaceEventSink, SurfaceRuntime};
