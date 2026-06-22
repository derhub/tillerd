//! Fs-backed keybinding store. File layout under `<fs_root>/config/keybindings/`:
//!   overrides.json            -- `{ "<action>": "<chord>" }` (user overrides only)
//!
//! The default keymap is static (compiled-in). The store only persists overrides.
//! `list` merges defaults with overrides (overrides win). `reset` removes an override.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::shared::{fs, Result};

/// A keybinding entry: action and its effective chord.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    fn ensure_dir(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
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
        Self::ensure_dir(&path)?;
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

    /// Effective keymap: defaults merged with overrides (overrides win), sorted by action.
    pub async fn list(&self) -> Result<Vec<KeybindingEntry>> {
        let overrides = self.read_overrides().await?;
        let mut merged = self.defaults.clone();
        for (action, chord) in overrides {
            merged.insert(action, chord);
        }
        let mut entries: Vec<KeybindingEntry> = merged
            .into_iter()
            .map(|(action, chord)| KeybindingEntry { action, chord })
            .collect();
        entries.sort_by(|a, b| a.action.cmp(&b.action));
        Ok(entries)
    }

    /// Resolve the chord(s) for a given action. Returns `None` if unbound.
    pub async fn resolve(&self, action: &str) -> Result<Option<String>> {
        let overrides = self.read_overrides().await?;
        if let Some(chord) = overrides.get(action) {
            return Ok(Some(chord.clone()));
        }
        Ok(self.defaults.get(action).cloned())
    }

    /// Resolve the action bound to a given chord (inverse lookup). Returns `None` if not bound.
    pub async fn resolve_action(&self, chord: &str) -> Result<Option<String>> {
        let overrides = self.read_overrides().await?;
        // Overrides take priority.
        for (action, c) in &overrides {
            if c == chord {
                return Ok(Some(action.clone()));
            }
        }
        // Fall back to defaults not shadowed by an override.
        let override_actions: std::collections::HashSet<_> = overrides.keys().cloned().collect();
        for (action, c) in &self.defaults {
            if c == chord && !override_actions.contains(action) {
                return Ok(Some(action.clone()));
            }
        }
        Ok(None)
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

    // Scenario: rebind sets an override; resolve returns the new chord
    #[tokio::test]
    async fn rebind_sets_override_and_resolve_returns_it() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.rebind("new-session", "ctrl+t").await.unwrap();
        let chord = s.resolve("new-session").await.unwrap();

        assert_eq!(chord.as_deref(), Some("ctrl+t"));
    }

    // Scenario: resolve returns default when no override is set
    #[tokio::test]
    async fn resolve_returns_default_when_no_override() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let chord = s.resolve("rename").await.unwrap();

        assert_eq!(chord.as_deref(), Some("F2"));
    }

    // Scenario: resolve returns None for an unbound action
    #[tokio::test]
    async fn resolve_returns_none_for_unbound_action() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let chord = s.resolve("not-a-real-action").await.unwrap();

        assert_eq!(chord, None);
    }

    // Scenario: reset removes an override, reverting to default
    #[tokio::test]
    async fn reset_reverts_to_default() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.rebind("rename", "ctrl+r").await.unwrap();
        s.reset("rename").await.unwrap();
        let chord = s.resolve("rename").await.unwrap();

        assert_eq!(chord.as_deref(), Some("F2"));
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

        assert_eq!(s.resolve("rename").await.unwrap().as_deref(), Some("F2"));
        assert_eq!(
            s.resolve("new-session").await.unwrap().as_deref(),
            Some("ctrl+n")
        );
    }

    // Scenario: list returns merged keymap sorted by action
    #[tokio::test]
    async fn list_returns_merged_sorted_keymap() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.rebind("rename", "ctrl+r").await.unwrap();
        let entries = s.list().await.unwrap();

        let actions: Vec<&str> = entries.iter().map(|e| e.action.as_str()).collect();
        // Should be sorted
        let mut sorted = actions.clone();
        sorted.sort();
        assert_eq!(actions, sorted);

        // Overridden rename must appear with the new chord
        let rename = entries.iter().find(|e| e.action == "rename").unwrap();
        assert_eq!(rename.chord, "ctrl+r");

        // Default close-surface must appear
        let close = entries
            .iter()
            .find(|e| e.action == "close-surface")
            .unwrap();
        assert_eq!(close.chord, "ctrl+w");
    }

    // Scenario: list returns defaults when no overrides exist
    #[tokio::test]
    async fn list_returns_defaults_with_no_overrides() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let entries = s.list().await.unwrap();

        assert_eq!(entries.len(), 3);
        let new_sess = entries.iter().find(|e| e.action == "new-session").unwrap();
        assert_eq!(new_sess.chord, "ctrl+n");
    }

    // Scenario: resolve_action inverse lookup returns the action for a chord
    #[tokio::test]
    async fn resolve_action_returns_action_for_chord() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let action = s.resolve_action("ctrl+n").await.unwrap();

        assert_eq!(action.as_deref(), Some("new-session"));
    }

    // Scenario: resolve_action returns override's action when chord is rebound
    #[tokio::test]
    async fn resolve_action_returns_overridden_action() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        // Rebind new-session to ctrl+t; ctrl+n is now unbound.
        s.rebind("new-session", "ctrl+t").await.unwrap();
        let by_old = s.resolve_action("ctrl+n").await.unwrap();
        let by_new = s.resolve_action("ctrl+t").await.unwrap();

        assert_eq!(by_old, None);
        assert_eq!(by_new.as_deref(), Some("new-session"));
    }

    // Scenario: external edit is reflected on next read (no caching)
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

        let chord = s.resolve("rename").await.unwrap();
        assert_eq!(chord.as_deref(), Some("ctrl+shift+r"));
    }
}
