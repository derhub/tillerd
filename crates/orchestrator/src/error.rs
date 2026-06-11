//! Typed error surface shared across persistence, supervision, and boot.

use thiserror::Error;

/// Errors the orchestrator surfaces during boot, supervision, and persistence.
///
/// Every boot failure is one of these typed variants; the orchestrator turns one
/// into a terminal `Failed { reason }` lifecycle event and never reports `ready`
/// after producing it (orchestrator-core: typed boot-failure surface).
#[derive(Debug, Error)]
pub enum OrchestratorError {
    /// The product store's recorded schema version is newer than this binary
    /// supports, so it cannot be served against (workspace-persistence).
    #[error("store schema version {found} is newer than this binary supports ({supported})")]
    StoreVersionTooNew {
        /// The schema version recorded in the store.
        found: u32,
        /// The highest schema version this binary can serve.
        supported: u32,
    },

    /// A persistence operation failed: opening, migrating, or querying the store.
    #[error("persistence: {0}")]
    Persistence(String),

    /// A supervised service could neither be adopted nor spawned to an available
    /// state (orchestrator-supervision: readiness gated on services).
    #[error("service '{service}' could not be made available: {reason}")]
    ServiceUnavailable {
        /// The supervised service that failed, e.g. `gate` or `daemon`.
        service: String,
        /// Why the service could not be made available.
        reason: String,
    },
}

/// Crate-wide result alias over [`OrchestratorError`].
pub type Result<T> = std::result::Result<T, OrchestratorError>;
