//! The single error registry. Each variant declares a stable, low-cardinality
//! telemetry code via `#[error_code("…")]`, generated into `code()` by the
//! `ErrorCode` derive; a variant missing the attribute is a compile error.
//! `Display`, `#[from]`, and the source chain stay with `thiserror`. There is no
//! `level`/`category` — every error logs at `ERROR`. Ids belong in the message,
//! never in a code.

use tillerd_custom_macro::ErrorCode;

#[derive(Debug, thiserror::Error, ErrorCode)]
pub enum Error {
    #[error("workspace not found: {0}")]
    #[error_code("workspace.not_found")]
    WorkspaceNotFound(String),

    #[error("the Default workspace cannot be deleted")]
    #[error_code("workspace.is_default")]
    WorkspaceIsDefault,

    #[error("workspace is already archived")]
    #[error_code("workspace.already_archived")]
    WorkspaceAlreadyArchived,

    #[error("workspace is not archived")]
    #[error_code("workspace.not_archived")]
    WorkspaceNotArchived,

    #[error("project not found: {0}")]
    #[error_code("project.not_found")]
    ProjectNotFound(String),

    #[error("the Unfiled project cannot be archived or deleted")]
    #[error_code("project.is_unfiled")]
    ProjectIsUnfiled,

    #[error("project must be archived before it can be hard-deleted")]
    #[error_code("project.not_archived")]
    ProjectNotArchived,

    #[error("project is already archived")]
    #[error_code("project.already_archived")]
    ProjectAlreadyArchived,

    #[error("session not found: {0}")]
    #[error_code("session.not_found")]
    SessionNotFound(String),

    #[error("session must be archived before it can be hard-deleted")]
    #[error_code("session.not_archived")]
    SessionNotArchived,

    #[error("session is already archived")]
    #[error_code("session.already_archived")]
    SessionAlreadyArchived,

    #[error("session {0} has running surfaces and cannot be archived")]
    #[error_code("session.not_idle")]
    SessionNotIdle(String),

    #[error("surface not found: {0}")]
    #[error_code("surface.not_found")]
    SurfaceNotFound(String),

    #[error("surface {0} is already associated with a different session")]
    #[error_code("surface.conflict")]
    SurfaceConflict(String),

    #[error("surface {surface} runtime error: {reason}")]
    #[error_code("surface.runtime")]
    SurfaceRuntime { surface: String, reason: String },

    #[error("command not found: {0}")]
    #[error_code("command.not_found")]
    CommandNotFound(String),

    #[error("a prebuilt {kind} is immutable")]
    #[error_code("prebuilt.immutable")]
    PrebuiltImmutable { kind: &'static str },

    #[error("launch template not found: {0}")]
    #[error_code("launch_template.not_found")]
    LaunchTemplateNotFound(String),

    #[error("template not found: {0}")]
    #[error_code("template.not_found")]
    TemplateNotFound(String),

    #[error("notification not found: {0}")]
    #[error_code("notification.not_found")]
    NotificationNotFound(String),

    #[error("profile not found: {0}")]
    #[error_code("profile.not_found")]
    ProfileNotFound(String),

    #[error("theme not found: {0}")]
    #[error_code("theme.not_found")]
    ThemeNotFound(String),

    #[error("setting not found: {0}")]
    #[error_code("setting.not_found")]
    SettingNotFound(String),

    #[error("launch spec is invalid: {0}")]
    #[error_code("launch_spec.invalid")]
    LaunchSpecInvalid(String),

    #[error("launch spec version {found} is newer than this binary supports ({supported})")]
    #[error_code("launch_spec.version_too_new")]
    LaunchSpecVersionTooNew { found: u32, supported: u32 },

    #[error("service '{service}' could not be made available: {reason}")]
    #[error_code("service.unavailable")]
    ServiceUnavailable { service: String, reason: String },

    #[error("invalid {field}: {reason}")]
    #[error_code("validation")]
    Validation { field: &'static str, reason: String },

    #[error(transparent)]
    #[error_code("db.error")]
    Db(#[from] sqlx::Error),

    #[error(transparent)]
    #[error_code("io.error")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    #[error_code("serde.error")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_is_low_cardinality_and_excludes_the_id() {
        let err = Error::WorkspaceNotFound("ws_9f3c".to_owned());
        assert_eq!(err.code(), "workspace.not_found");
    }

    #[test]
    fn the_id_appears_in_the_message_not_the_code() {
        let err = Error::WorkspaceNotFound("ws_9f3c".to_owned());
        assert!(err.to_string().contains("ws_9f3c"));
    }

    #[test]
    fn a_sqlx_error_converts_via_from_with_a_transparent_message() {
        let inner = sqlx::Error::RowNotFound;
        let inner_message = inner.to_string();
        let err: Error = inner.into();
        assert_eq!(err.code(), "db.error");
        assert_eq!(err.to_string(), inner_message);
    }

    #[test]
    fn an_io_error_converts_via_from() {
        let err: Error = std::io::Error::from(std::io::ErrorKind::NotFound).into();
        assert_eq!(err.code(), "io.error");
    }

    #[test]
    fn a_serde_error_converts_via_from() {
        let serde_err = serde_json::from_str::<i32>("not json").unwrap_err();
        let err: Error = serde_err.into();
        assert_eq!(err.code(), "serde.error");
    }
}
