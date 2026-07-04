use serde::Serialize;

/// Flat read model for a project row. Serializes to the SDK `Project` wire shape.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub id: String,
    pub name: String,
    pub source_kind: String,
    pub root_path: Option<String>,
    pub workspace_id: String,
    pub status: String,
    pub pinned: bool,
}
