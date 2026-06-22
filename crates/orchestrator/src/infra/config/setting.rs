//! Fs-backed settings store. Each scope maps to one JSON file:
//!   <fs_root>/config/settings/global.json
//!   <fs_root>/config/settings/project/<project_id>.json
//!
//! Values are stored as `{ "<key>": <json_value> }` maps.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::entities::setting::{SettingEntry, SettingScope};
use crate::shared::{fs, Result};

/// File-backed settings store scoped to a config root directory.
pub struct SettingStore {
    root: PathBuf,
}

impl SettingStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn global_path(&self) -> PathBuf {
        self.root
            .join("config")
            .join("settings")
            .join("global.json")
    }

    fn project_path(&self, project_id: &str) -> PathBuf {
        self.root
            .join("config")
            .join("settings")
            .join("project")
            .join(format!("{project_id}.json"))
    }

    fn scope_path(&self, scope: &SettingScope) -> PathBuf {
        match scope {
            SettingScope::Global => self.global_path(),
            SettingScope::Project(pid) => self.project_path(pid.as_str()),
        }
    }

    async fn ensure_parent(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    async fn read_map(path: &Path) -> Result<HashMap<String, String>> {
        match fs::read_string(path).await {
            Ok(s) => Ok(serde_json::from_str(&s)?),
            Err(_) => Ok(HashMap::new()),
        }
    }

    async fn write_map(path: &Path, map: &HashMap<String, String>) -> Result<()> {
        Self::ensure_parent(path).await?;
        let s = serde_json::to_string_pretty(map)?;
        fs::write_string(path, &s).await
    }

    /// Set (or overwrite) a setting value at the given scope.
    pub async fn apply(&self, scope: &SettingScope, key: &str, value_json: &str) -> Result<()> {
        let path = self.scope_path(scope);
        let mut map = Self::read_map(&path).await?;
        map.insert(key.to_owned(), value_json.to_owned());
        Self::write_map(&path, &map).await
    }

    /// Remove a setting override at the given scope. Returns `Ok(())` whether it existed or not.
    pub async fn reset(&self, scope: &SettingScope, key: &str) -> Result<()> {
        let path = self.scope_path(scope);
        let mut map = Self::read_map(&path).await?;
        map.remove(key);
        Self::write_map(&path, &map).await
    }

    /// Get a raw setting value at the given scope. Returns `None` if absent.
    pub async fn get(&self, scope: &SettingScope, key: &str) -> Result<Option<String>> {
        let path = self.scope_path(scope);
        let map = Self::read_map(&path).await?;
        Ok(map.get(key).cloned())
    }

    /// List all overrides at a scope, sorted by key.
    pub async fn list(&self, scope: &SettingScope) -> Result<Vec<SettingEntry>> {
        let path = self.scope_path(scope);
        let map = Self::read_map(&path).await?;
        let mut entries: Vec<SettingEntry> = map
            .into_iter()
            .map(|(key, value_json)| SettingEntry { key, value_json })
            .collect();
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(entries)
    }

    /// Effective value for a project: project-scoped override if present, else global, else `None`.
    pub async fn resolve(
        &self,
        project_id: &crate::entities::project::ProjectId,
        key: &str,
    ) -> Result<Option<String>> {
        if let Some(v) = self
            .get(&SettingScope::Project(project_id.clone()), key)
            .await?
        {
            return Ok(Some(v));
        }
        self.get(&SettingScope::Global, key).await
    }

    /// Effective settings map for a project: global defaults overridden by project-scoped values.
    pub async fn resolve_all(
        &self,
        project_id: &crate::entities::project::ProjectId,
    ) -> Result<Vec<SettingEntry>> {
        let global_path = self.global_path();
        let project_path = self.project_path(project_id.as_str());

        let mut map = Self::read_map(&global_path).await?;
        let project_map = Self::read_map(&project_path).await?;

        for (k, v) in project_map {
            map.insert(k, v);
        }

        let mut entries: Vec<SettingEntry> = map
            .into_iter()
            .map(|(key, value_json)| SettingEntry { key, value_json })
            .collect();
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::project::ProjectId;
    use tempfile::TempDir;

    fn store(dir: &TempDir) -> SettingStore {
        SettingStore::new(dir.path())
    }

    fn pid() -> ProjectId {
        ProjectId::new("test-project".to_owned())
    }

    // Scenario: apply then get round-trips a global setting
    #[tokio::test]
    async fn apply_and_get_global_setting_round_trips() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.apply(&SettingScope::Global, "theme", r#""dark""#)
            .await
            .unwrap();
        let v = s.get(&SettingScope::Global, "theme").await.unwrap();

        assert_eq!(v.as_deref(), Some(r#""dark""#));
    }

    // Scenario: apply then get round-trips a project-scoped setting
    #[tokio::test]
    async fn apply_and_get_project_setting_round_trips() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.apply(&SettingScope::Project(pid()), "font-size", "14")
            .await
            .unwrap();
        let v = s
            .get(&SettingScope::Project(pid()), "font-size")
            .await
            .unwrap();

        assert_eq!(v.as_deref(), Some("14"));
    }

    // Scenario: get returns None for absent key
    #[tokio::test]
    async fn get_returns_none_for_absent_key() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let v = s.get(&SettingScope::Global, "missing").await.unwrap();

        assert_eq!(v, None);
    }

    // Scenario: apply overwrites existing value without duplicates
    #[tokio::test]
    async fn apply_overwrites_existing_value() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.apply(&SettingScope::Global, "k", r#""v1""#)
            .await
            .unwrap();
        s.apply(&SettingScope::Global, "k", r#""v2""#)
            .await
            .unwrap();
        let v = s.get(&SettingScope::Global, "k").await.unwrap();
        let listed = s.list(&SettingScope::Global).await.unwrap();

        assert_eq!(v.as_deref(), Some(r#""v2""#));
        assert_eq!(listed.iter().filter(|e| e.key == "k").count(), 1);
    }

    // Scenario: reset removes the override; absent key is a no-op
    #[tokio::test]
    async fn reset_removes_override() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.apply(&SettingScope::Global, "k", r#""v""#).await.unwrap();
        s.reset(&SettingScope::Global, "k").await.unwrap();

        assert_eq!(s.get(&SettingScope::Global, "k").await.unwrap(), None);
    }

    #[tokio::test]
    async fn reset_absent_key_is_ok() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let result = s.reset(&SettingScope::Global, "never-set").await;

        assert!(result.is_ok());
    }

    // Scenario: list returns all overrides sorted by key
    #[tokio::test]
    async fn list_returns_entries_sorted_by_key() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.apply(&SettingScope::Global, "z-key", "1").await.unwrap();
        s.apply(&SettingScope::Global, "a-key", "2").await.unwrap();
        s.apply(&SettingScope::Global, "m-key", "3").await.unwrap();
        let entries = s.list(&SettingScope::Global).await.unwrap();

        let keys: Vec<&str> = entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, vec!["a-key", "m-key", "z-key"]);
    }

    // Scenario: resolve returns project-scoped value when present
    #[tokio::test]
    async fn resolve_returns_project_override_over_global() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.apply(&SettingScope::Global, "env", r#""global""#)
            .await
            .unwrap();
        s.apply(&SettingScope::Project(pid()), "env", r#""project""#)
            .await
            .unwrap();
        let v = s.resolve(&pid(), "env").await.unwrap();

        assert_eq!(v.as_deref(), Some(r#""project""#));
    }

    // Scenario: resolve falls back to global when no project override
    #[tokio::test]
    async fn resolve_falls_back_to_global() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.apply(&SettingScope::Global, "env", r#""global""#)
            .await
            .unwrap();
        let v = s.resolve(&pid(), "env").await.unwrap();

        assert_eq!(v.as_deref(), Some(r#""global""#));
    }

    // Scenario: resolve returns None when absent everywhere
    #[tokio::test]
    async fn resolve_returns_none_when_absent_everywhere() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let v = s.resolve(&pid(), "nope").await.unwrap();

        assert_eq!(v, None);
    }

    // Scenario: resolve_all merges global and project, project wins on overlap
    #[tokio::test]
    async fn resolve_all_merges_global_and_project() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.apply(&SettingScope::Global, "a", r#""ga""#)
            .await
            .unwrap();
        s.apply(&SettingScope::Global, "b", r#""gb""#)
            .await
            .unwrap();
        s.apply(&SettingScope::Project(pid()), "b", r#""pb""#)
            .await
            .unwrap();
        s.apply(&SettingScope::Project(pid()), "c", r#""pc""#)
            .await
            .unwrap();
        let entries = s.resolve_all(&pid()).await.unwrap();

        let map: HashMap<_, _> = entries
            .iter()
            .map(|e| (e.key.as_str(), e.value_json.as_str()))
            .collect();
        assert_eq!(map["a"], r#""ga""#);
        assert_eq!(map["b"], r#""pb""#); // project overrides global
        assert_eq!(map["c"], r#""pc""#);
    }

    // Scenario: external config edit is picked up (no caching — reads from disk each call)
    #[tokio::test]
    async fn external_edit_is_reflected_on_next_read() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.apply(&SettingScope::Global, "k", r#""old""#)
            .await
            .unwrap();

        // Simulate external edit by writing directly to disk.
        let path = dir
            .path()
            .join("config")
            .join("settings")
            .join("global.json");
        let raw = std::fs::read_to_string(&path).unwrap();
        let mut map: HashMap<String, String> = serde_json::from_str(&raw).unwrap();
        map.insert("k".to_owned(), r#""new""#.to_owned());
        std::fs::write(&path, serde_json::to_string_pretty(&map).unwrap()).unwrap();

        let v = s.get(&SettingScope::Global, "k").await.unwrap();
        assert_eq!(v.as_deref(), Some(r#""new""#));
    }
}
