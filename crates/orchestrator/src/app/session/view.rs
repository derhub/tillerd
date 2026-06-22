use serde::Serialize;

/// Flat read model for a session row. Serializes to the SDK `Session` wire shape
/// (the same camelCase JSON the prior `SessionResponse` host struct produced).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub title_source: String,
    pub created_at: String,
}

/// Read model for a session's launch spec (the migrated recipe + placements).
///
/// `GetLaunchSpec` deserializes and migrates the stored `spec_json` blob, so this
/// View is not a flat row map; it wraps the typed spec and serializes identically.
#[derive(Debug, Clone, Serialize)]
pub struct LaunchSpecView(pub crate::entities::launch_spec::LaunchSpec);
