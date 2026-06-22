use tillerd_custom_macro::ErrorCode;

#[derive(ErrorCode)]
#[allow(dead_code)]
enum Sample {
    #[error_code("workspace.not_found")]
    WorkspaceNotFound(String),
    #[error_code("workspace.is_default")]
    WorkspaceIsDefault,
    #[error_code("validation")]
    Validation { field: &'static str, reason: String },
}

#[test]
fn code_returns_declared_string_for_tuple_variant() {
    let err = Sample::WorkspaceNotFound("ws_1".to_owned());
    assert_eq!(err.code(), "workspace.not_found");
}

#[test]
fn code_returns_declared_string_for_unit_variant() {
    assert_eq!(Sample::WorkspaceIsDefault.code(), "workspace.is_default");
}

#[test]
fn code_returns_declared_string_for_struct_variant() {
    let err = Sample::Validation {
        field: "name",
        reason: "empty".to_owned(),
    };
    assert_eq!(err.code(), "validation");
}
