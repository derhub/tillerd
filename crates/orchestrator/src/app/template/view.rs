use serde::Serialize;

/// Flat read model for a project-bound launch template row.
///
/// Decoded straight from the `launch_template` table via `query_as`.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LaunchTemplateView {
    pub id: String,
    pub project_id: String,
    pub spec_version: i64,
    pub spec_json: String,
}

/// Flat read model for a portable library template.
///
/// Built from the filesystem template index + bundle (not table-mapped), so it is
/// assembled by hand in the query handler rather than via `query_as`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TemplateView {
    pub id: String,
    pub name: String,
    pub origin: String,
    pub pinned: bool,
    pub spec_version: u32,
    pub spec_json: String,
}
