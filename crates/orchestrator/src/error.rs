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

    #[error("launch spec is invalid: {0}")]
    LaunchSpecInvalid(String),

    #[error("launch spec version {found} is newer than this binary supports ({supported})")]
    LaunchSpecVersionTooNew { found: u32, supported: u32 },

    #[error("command not found: {0}")]
    CommandNotFound(String),

    #[error("launch template not found: {0}")]
    LaunchTemplateNotFound(String),

    #[error("worktree step failed: {0}")]
    WorktreeStepFailed(String),

    #[error("worktree not found: {0}")]
    WorktreeNotFound(String),
}

pub type Result<T> = std::result::Result<T, OrchestratorError>;
