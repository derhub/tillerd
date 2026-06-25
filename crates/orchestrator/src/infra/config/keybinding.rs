//! Fs-backed keybinding store. File layout under `<fs_root>/config/keybindings/`:
//!   overrides.json            -- `{ "<action>": "<chord>" }` (user overrides only)
//!
//! The default keymap is static (compiled-in). The store only persists overrides.
//! `reset` removes a single override; `reset_all` clears all overrides.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::shared::{fs, Result};

/// A keybinding entry: action and its effective chord.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct KeybindingEntry {
    pub action: String,
    pub chord: String,
}

/// Fs-backed keybinding store.
pub struct KeybindingStore {
    root: PathBuf,
    /// Static default keymap: action -> chord.
    defaults: HashMap<String, String>,
}

impl KeybindingStore {
    /// Create a store with the given config root and default keymap.
    pub fn new(root: impl Into<PathBuf>, defaults: HashMap<String, String>) -> Self {
        Self {
            root: root.into(),
            defaults,
        }
    }

    fn overrides_path(&self) -> PathBuf {
        self.root
            .join("config")
            .join("keybindings")
            .join("overrides.json")
    }

    async fn ensure_dir(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        Ok(())
    }

    async fn read_overrides(&self) -> Result<HashMap<String, String>> {
        let path = self.overrides_path();
        match fs::read_string(&path).await {
            Ok(s) => Ok(serde_json::from_str(&s)?),
            Err(_) => Ok(HashMap::new()),
        }
    }

    async fn write_overrides(&self, overrides: &HashMap<String, String>) -> Result<()> {
        let path = self.overrides_path();
        Self::ensure_dir(&path).await?;
        let s = serde_json::to_string_pretty(overrides)?;
        fs::write_string(&path, &s).await
    }

    /// Rebind an action to a chord (creates or overwrites the override).
    pub async fn rebind(&self, action: &str, chord: &str) -> Result<()> {
        let mut overrides = self.read_overrides().await?;
        overrides.insert(action.to_owned(), chord.to_owned());
        self.write_overrides(&overrides).await
    }

    /// Reset one action to its default, removing the user override.
    pub async fn reset(&self, action: &str) -> Result<()> {
        let mut overrides = self.read_overrides().await?;
        overrides.remove(action);
        self.write_overrides(&overrides).await
    }

    /// Reset the entire keymap to defaults by clearing all overrides.
    pub async fn reset_all(&self) -> Result<()> {
        self.write_overrides(&HashMap::new()).await
    }

    /// The compiled-in default keymap.
    pub fn defaults(&self) -> &HashMap<String, String> {
        &self.defaults
    }

    /// The persisted user overrides (action -> chord).
    pub async fn overrides(&self) -> Result<HashMap<String, String>> {
        self.read_overrides().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn defaults() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("new-session".to_owned(), "ctrl+n".to_owned());
        m.insert("close-surface".to_owned(), "ctrl+w".to_owned());
        m.insert("rename".to_owned(), "F2".to_owned());
        m
    }

    fn store(dir: &TempDir) -> KeybindingStore {
        KeybindingStore::new(dir.path(), defaults())
    }

    // Scenario: rebind persists an override; overrides() returns it
    #[tokio::test]
    async fn rebind_persists_override() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.rebind("new-session", "ctrl+t").await.unwrap();
        let overrides = s.overrides().await.unwrap();

        assert_eq!(
            overrides.get("new-session").map(String::as_str),
            Some("ctrl+t")
        );
    }

    // Scenario: defaults() returns the compiled-in keymap
    #[test]
    fn defaults_returns_static_keymap() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        assert_eq!(s.defaults().get("rename").map(String::as_str), Some("F2"));
        assert_eq!(
            s.defaults().get("new-session").map(String::as_str),
            Some("ctrl+n")
        );
    }

    // Scenario: reset removes an override; overrides() omits it after reset
    #[tokio::test]
    async fn reset_removes_override() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.rebind("rename", "ctrl+r").await.unwrap();
        s.reset("rename").await.unwrap();

        assert!(!s.overrides().await.unwrap().contains_key("rename"));
    }

    // Scenario: reset on absent override is ok
    #[tokio::test]
    async fn reset_absent_override_is_ok() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        assert!(s.reset("never-overridden").await.is_ok());
    }

    // Scenario: reset_all clears all overrides
    #[tokio::test]
    async fn reset_all_clears_all_overrides() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.rebind("rename", "ctrl+r").await.unwrap();
        s.rebind("new-session", "ctrl+t").await.unwrap();
        s.reset_all().await.unwrap();

        assert!(s.overrides().await.unwrap().is_empty());
    }

    // Scenario: external edit is reflected on next overrides() call (no caching)
    #[tokio::test]
    async fn external_edit_is_reflected_on_next_read() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.rebind("rename", "ctrl+r").await.unwrap();

        // Simulate external edit.
        let path = dir
            .path()
            .join("config")
            .join("keybindings")
            .join("overrides.json");
        let raw = std::fs::read_to_string(&path).unwrap();
        let mut overrides: HashMap<String, String> = serde_json::from_str(&raw).unwrap();
        overrides.insert("rename".to_owned(), "ctrl+shift+r".to_owned());
        std::fs::write(&path, serde_json::to_string_pretty(&overrides).unwrap()).unwrap();

        let loaded = s.overrides().await.unwrap();
        assert_eq!(
            loaded.get("rename").map(String::as_str),
            Some("ctrl+shift+r")
        );
    }
}
