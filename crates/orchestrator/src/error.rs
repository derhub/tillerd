use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("store schema version {found} is newer than this binary supports ({supported})")]
    StoreVersionTooNew { found: u32, supported: u32 },

    #[error("persistence: {0}")]
    Persistence(String),

    #[error("service '{service}' could not be made available: {reason}")]
    ServiceUnavailable { service: String, reason: String },

    #[error("surface '{surface}': {reason}")]
    Surface { surface: String, reason: String },

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("the Unfiled project cannot be archived or deleted")]
    ProjectIsUnfiled,

    #[error("project must be archived before it can be hard-deleted")]
    ProjectNotArchived,

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("session must be archived before it can be hard-deleted")]
    SessionNotArchived,

    #[error("surface {0} is already associated with a different session")]
    SurfaceConflict(String),
}

pub type Result<T> = std::result::Result<T, OrchestratorError>;
