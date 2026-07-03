use serde::Serialize;

/// Flat read model for a workspace row. Serializes to the SDK `Workspace` wire shape.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceView {
    pub id: String,
    pub name: String,
    pub status: String,
}

/// Per-workspace rollup of surface runtime state (ADR-0044): derived at query
/// time from the persisted surface status, never a stored domain field.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceActivityView {
    pub workspace_id: String,
    pub running: u32,
    pub failed: u32,
}
