use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("store schema version {found} is newer than this binary supports ({supported})")]
    StoreVersionTooNew { found: u32, supported: u32 },

    #[error("persistence: {0}")]
    Persistence(String),

    #[error("service '{service}' could not be made available: {reason}")]
    ServiceUnavailable { service: String, reason: String },
}

pub type Result<T> = std::result::Result<T, OrchestratorError>;
