//! Settings store.

use crate::entities::{ProjectId, SettingEntry, SettingScope};
use crate::error::Result;
use crate::store::backend::Backend;

/// Operational store for scoped settings.
#[derive(Clone)]
pub struct Settings {
    backend: Backend,
}

impl Settings {
    pub fn new(backend: Backend) -> Self {
        Self { backend }
    }

    pub async fn get(&self, scope: SettingScope, key: String) -> Result<Option<String>> {
        self.backend.get_setting(scope, key).await
    }

    pub async fn set(&self, scope: SettingScope, key: String, value_json: String) -> Result<()> {
        self.backend.set_setting(scope, key, value_json).await
    }

    pub async fn list(&self, scope: SettingScope) -> Result<Vec<SettingEntry>> {
        self.backend.list_settings(scope).await
    }

    /// The project-scoped value if present, else the global value, else `None`.
    pub async fn resolve(&self, project_id: ProjectId, key: String) -> Result<Option<String>> {
        self.backend.resolve_setting(project_id, key).await
    }
}
