//! Setting entity: a JSON-encoded value stored under a global or project scope.

use super::project::ProjectId;

/// Scope a setting is stored under: app-global, or bound to a specific project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingScope {
    Global,
    Project(ProjectId),
}

impl SettingScope {
    /// The `(scope, project_id)` column pair for the `setting` table. Global uses an
    /// empty `project_id` sentinel -- never NULL -- so the composite primary key
    /// `(scope, project_id, key)` stays unique and upsert works (SQLite treats NULLs
    /// as distinct, which would defeat both).
    pub fn columns(&self) -> (&'static str, &str) {
        match self {
            SettingScope::Global => ("global", ""),
            SettingScope::Project(id) => ("project", id.as_str()),
        }
    }
}

/// A stored setting: its key and JSON-encoded value.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct SettingEntry {
    pub key: String,
    pub value_json: String,
}
