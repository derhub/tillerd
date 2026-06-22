use serde::Serialize;

/// Flat read model for a project row. Serializes to the SDK `Project` wire shape.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub id: String,
    pub name: String,
    pub source_kind: String,
    pub root_path: Option<String>,
    pub workspace_id: String,
}
