//! Fs-backed theme store. File layout under `<fs_root>/config/themes/`:
//!   active.json               -- `{ "active": "<theme_id>" }`
//!   <theme_id>.json           -- `{ "id": "...", "name": "...", "origin": "prebuilt"|"custom", ... }`

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::shared::{fs, Result};

/// Theme origin: prebuilt (immutable) or custom (user-added).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeOrigin {
    Prebuilt,
    Custom,
}

/// A stored theme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub origin: ThemeOrigin,
    /// Opaque theme data (colors, fonts, etc.) as raw JSON.
    #[serde(default)]
    pub data_json: Option<String>,
}

/// Fs-backed theme store.
pub struct ThemeStore {
    root: PathBuf,
}

impl ThemeStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn themes_dir(&self) -> PathBuf {
        self.root.join("config").join("themes")
    }

    fn theme_path(&self, id: &str) -> PathBuf {
        self.themes_dir().join(format!("{id}.json"))
    }

    fn active_path(&self) -> PathBuf {
        self.themes_dir().join("active.json")
    }

    fn ensure_dir(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    /// Import (register) a theme. For prebuilt themes use `ThemeOrigin::Prebuilt`.
    pub async fn import(&self, theme: &Theme) -> Result<()> {
        let path = self.theme_path(&theme.id);
        Self::ensure_dir(&path)?;
        let s = serde_json::to_string_pretty(theme)?;
        fs::write_string(&path, &s).await
    }

    /// Get a theme by id. Returns `None` if not found.
    pub async fn get(&self, id: &str) -> Result<Option<Theme>> {
        let path = self.theme_path(id);
        match fs::read_string(&path).await {
            Ok(s) => Ok(Some(serde_json::from_str(&s)?)),
            Err(_) => Ok(None),
        }
    }

    /// List all themes (prebuilt + custom), sorted by id.
    pub async fn list(&self) -> Result<Vec<Theme>> {
        let dir = self.themes_dir();
        let entries = fs::list_entries(&dir).await?;
        let mut themes = Vec::new();
        for path in entries {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string());
            match name.as_deref() {
                Some("active.json") => continue,
                Some(n) if n.ends_with(".json") => {}
                _ => continue,
            }
            if let Ok(s) = fs::read_string(&path).await {
                if let Ok(t) = serde_json::from_str::<Theme>(&s) {
                    themes.push(t);
                }
            }
        }
        themes.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(themes)
    }

    /// Delete a theme file by id. No origin check; callers enforce invariants.
    pub async fn delete(&self, id: &str) -> Result<()> {
        let path = self.theme_path(id);
        fs::delete(&path).await
    }

    /// Export a theme bundle as JSON bytes.
    pub async fn export(&self, id: &str) -> Result<Option<Vec<u8>>> {
        let path = self.theme_path(id);
        match fs::read_bytes(&path).await {
            Ok(b) => Ok(Some(b)),
            Err(_) => Ok(None),
        }
    }

    /// Set the active theme by id.
    pub async fn set_active(&self, id: &str) -> Result<()> {
        let path = self.active_path();
        Self::ensure_dir(&path)?;
        let v = ActiveFile {
            active: id.to_owned(),
        };
        let s = serde_json::to_string_pretty(&v)?;
        fs::write_string(&path, &s).await
    }

    /// Get the active theme id. Returns `None` if not set.
    pub async fn active_id(&self) -> Result<Option<String>> {
        let path = self.active_path();
        match fs::read_string(&path).await {
            Ok(s) => {
                let v: ActiveFile = serde_json::from_str(&s)?;
                Ok(Some(v.active))
            }
            Err(_) => Ok(None),
        }
    }

    /// Get the active theme entity. Returns `None` if not set or not found.
    pub async fn get_active(&self) -> Result<Option<Theme>> {
        match self.active_id().await? {
            Some(id) => self.get(&id).await,
            None => Ok(None),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ActiveFile {
    active: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store(dir: &TempDir) -> ThemeStore {
        ThemeStore::new(dir.path())
    }

    fn custom_theme(id: &str) -> Theme {
        Theme {
            id: id.to_owned(),
            name: id.to_owned(),
            origin: ThemeOrigin::Custom,
            data_json: None,
        }
    }

    fn prebuilt_theme(id: &str) -> Theme {
        Theme {
            id: id.to_owned(),
            name: id.to_owned(),
            origin: ThemeOrigin::Prebuilt,
            data_json: None,
        }
    }

    // Scenario: import and get round-trip
    #[tokio::test]
    async fn import_and_get_round_trips() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        let theme = custom_theme("solarized");

        s.import(&theme).await.unwrap();
        let loaded = s.get("solarized").await.unwrap();

        assert_eq!(loaded, Some(theme));
    }

    // Scenario: get returns None for absent theme
    #[tokio::test]
    async fn get_returns_none_for_absent_theme() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        assert_eq!(s.get("nope").await.unwrap(), None);
    }

    // Scenario: list returns prebuilt and custom themes sorted by id
    #[tokio::test]
    async fn list_returns_all_themes_sorted_by_id() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.import(&prebuilt_theme("z-builtin")).await.unwrap();
        s.import(&custom_theme("a-custom")).await.unwrap();
        let themes = s.list().await.unwrap();
        let ids: Vec<&str> = themes.iter().map(|t| t.id.as_str()).collect();

        assert_eq!(ids, vec!["a-custom", "z-builtin"]);
    }

    // Scenario: delete removes a theme by id
    #[tokio::test]
    async fn delete_removes_theme() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.import(&custom_theme("my-theme")).await.unwrap();
        s.delete("my-theme").await.unwrap();

        assert_eq!(s.get("my-theme").await.unwrap(), None);
    }

    // Scenario: delete absent theme is ok
    #[tokio::test]
    async fn delete_absent_theme_is_ok() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        assert!(s.delete("never-existed").await.is_ok());
    }

    // Scenario: set_active / get_active / active_id round-trip
    #[tokio::test]
    async fn set_active_and_get_active_round_trip() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.import(&prebuilt_theme("default")).await.unwrap();
        s.set_active("default").await.unwrap();
        let active = s.get_active().await.unwrap();

        assert_eq!(active.as_ref().map(|t| t.id.as_str()), Some("default"));
        assert_eq!(s.active_id().await.unwrap().as_deref(), Some("default"));
    }

    // Scenario: active_id returns None before any active is set
    #[tokio::test]
    async fn active_id_returns_none_when_not_set() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        assert_eq!(s.active_id().await.unwrap(), None);
    }

    // Scenario: switching active theme changes get_active
    #[tokio::test]
    async fn switching_active_theme_changes_get_active() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.import(&prebuilt_theme("light")).await.unwrap();
        s.import(&prebuilt_theme("dark")).await.unwrap();
        s.set_active("light").await.unwrap();
        s.set_active("dark").await.unwrap();

        assert_eq!(
            s.get_active()
                .await
                .unwrap()
                .as_ref()
                .map(|t| t.id.as_str()),
            Some("dark")
        );
    }

    // Scenario: export returns the theme file bytes
    #[tokio::test]
    async fn export_returns_theme_bytes() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        let theme = custom_theme("exportable");

        s.import(&theme).await.unwrap();
        let bytes = s.export("exportable").await.unwrap();

        assert!(bytes.is_some());
        let t: Theme = serde_json::from_slice(&bytes.unwrap()).unwrap();
        assert_eq!(t.id, "exportable");
    }

    // Scenario: export returns None for absent theme
    #[tokio::test]
    async fn export_returns_none_for_absent_theme() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        assert_eq!(s.export("nope").await.unwrap(), None);
    }

    // Scenario: external edit is reflected on next read (no caching)
    #[tokio::test]
    async fn external_edit_is_reflected_on_next_read() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.import(&custom_theme("t1")).await.unwrap();

        let path = dir.path().join("config").join("themes").join("t1.json");
        let raw = std::fs::read_to_string(&path).unwrap();
        let mut t: Theme = serde_json::from_str(&raw).unwrap();
        t.name = "Updated Name".to_owned();
        std::fs::write(&path, serde_json::to_string_pretty(&t).unwrap()).unwrap();

        let loaded = s.get("t1").await.unwrap().unwrap();
        assert_eq!(loaded.name, "Updated Name");
    }
}
