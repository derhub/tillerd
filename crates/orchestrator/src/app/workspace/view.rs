use serde::Serialize;

/// Flat read model for a workspace row. Serializes to the SDK `Workspace` wire shape.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceView {
    pub id: String,
    pub name: String,
}
