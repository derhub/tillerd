use super::*;

impl MemoryBackend {
    pub(crate) fn get_setting(&self, scope: &SettingScope, key: &str) -> Result<Option<String>> {
        let (scope_col, project_col) = scope.columns();
        Ok(self
            .inner
            .lock()
            .unwrap()
            .settings
            .get(&(
                scope_col.to_string(),
                project_col.to_string(),
                key.to_string(),
            ))
            .cloned())
    }

    /// Resolve a key for a project: the project-scoped value if present, else the global value.
    pub(crate) fn resolve_setting(
        &self,
        project_id: &ProjectId,
        key: &str,
    ) -> Result<Option<String>> {
        if let Some(v) = self.get_setting(&SettingScope::Project(project_id.clone()), key)? {
            return Ok(Some(v));
        }
        self.get_setting(&SettingScope::Global, key)
    }

    pub(crate) fn set_setting(
        &self,
        scope: &SettingScope,
        key: &str,
        value_json: &str,
    ) -> Result<()> {
        let (scope_col, project_col) = scope.columns();
        self.inner.lock().unwrap().settings.insert(
            (
                scope_col.to_string(),
                project_col.to_string(),
                key.to_string(),
            ),
            value_json.to_string(),
        );
        Ok(())
    }

    pub(crate) fn list_settings(&self, scope: &SettingScope) -> Result<Vec<SettingEntry>> {
        let (scope_col, project_col) = scope.columns();
        let inner = self.inner.lock().unwrap();
        let mut entries: Vec<SettingEntry> = inner
            .settings
            .iter()
            .filter(|((s, p, _), _)| s == scope_col && p == project_col)
            .map(|((_, _, key), value_json)| SettingEntry {
                key: key.clone(),
                value_json: value_json.clone(),
            })
            .collect();
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(entries)
    }
}
