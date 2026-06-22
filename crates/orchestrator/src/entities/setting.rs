//! Setting entity: a JSON-encoded value stored under a global or project scope.

use super::project::ProjectId;

/// Scope a setting is stored under: app-global, or bound to a specific project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingScope {
    Global,
    Project(ProjectId),
}

/// A stored setting: its key and JSON-encoded value.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct SettingEntry {
    pub key: String,
    pub value_json: String,
}
