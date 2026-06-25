//! Fs-backed profile store. File layout under `<fs_root>/config/profiles/`:
//!   active.json               -- `{ "active": "<profile_id>" }`
//!   <profile_id>.json         -- `{ "id": "...", "name": "...", "settings": {...} }`

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::shared::{fs, Result};

/// A stored profile: id, name, and a map of setting overrides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Profile {
    pub id: String,
    pub name: String,
    /// Setting overrides keyed by setting key. Values are JSON-encoded.
    #[serde(default)]
    pub settings: HashMap<String, String>,
}

/// Fs-backed profile store.
pub struct ProfileStore {
    root: PathBuf,
}

impl ProfileStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn profiles_dir(&self) -> PathBuf {
        self.root.join("config").join("profiles")
    }

    fn profile_path(&self, id: &str) -> PathBuf {
        self.profiles_dir().join(format!("{id}.json"))
    }

    fn active_path(&self) -> PathBuf {
        self.profiles_dir().join("active.json")
    }

    async fn ensure_dir(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        Ok(())
    }

    /// Create a new profile. Returns the created profile.
    pub async fn create(&self, id: &str, name: &str) -> Result<Profile> {
        let profile = Profile {
            id: id.to_owned(),
            name: name.to_owned(),
            settings: HashMap::new(),
        };
        self.save(&profile).await?;
        Ok(profile)
    }

    /// Save (create or overwrite) a profile.
    pub async fn save(&self, profile: &Profile) -> Result<()> {
        let path = self.profile_path(&profile.id);
        Self::ensure_dir(&path).await?;
        let s = serde_json::to_string_pretty(profile)?;
        fs::write_string(&path, &s).await
    }

    /// Load a profile by id. Returns `None` if not found.
    pub async fn get(&self, id: &str) -> Result<Option<Profile>> {
        let path = self.profile_path(id);
        match fs::read_string(&path).await {
            Ok(s) => Ok(Some(serde_json::from_str(&s)?)),
            Err(_) => Ok(None),
        }
    }

    /// List all profiles, sorted by id.
    pub async fn list(&self) -> Result<Vec<Profile>> {
        let dir = self.profiles_dir();
        let entries = fs::list_entries(&dir).await?;
        let mut profiles = Vec::new();
        for path in entries {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string());
            match name.as_deref() {
                Some("active.json") => continue,
                Some(n) if n.ends_with(".json") => {}
                _ => continue,
            }
            if let Ok(s) = fs::read_string(&path).await {
                if let Ok(p) = serde_json::from_str::<Profile>(&s) {
                    profiles.push(p);
                }
            }
        }
        profiles.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(profiles)
    }

    /// Delete a profile. Returns `Ok(())` whether it existed or not.
    pub async fn delete(&self, id: &str) -> Result<()> {
        let path = self.profile_path(id);
        fs::delete(&path).await
    }

    /// Get the active profile id. Returns `None` if no active profile is set.
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

    /// Set the active profile by id.
    pub async fn set_active(&self, id: &str) -> Result<()> {
        let path = self.active_path();
        Self::ensure_dir(&path).await?;
        let v = ActiveFile {
            active: id.to_owned(),
        };
        let s = serde_json::to_string_pretty(&v)?;
        fs::write_string(&path, &s).await
    }

    /// Get the active profile entity (loads its settings). Returns `None` if not set or not found.
    pub async fn get_active(&self) -> Result<Option<Profile>> {
        match self.active_id().await? {
            Some(id) => self.get(&id).await,
            None => Ok(None),
        }
    }

    /// Duplicate a profile under a new id/name. Returns the new profile.
    pub async fn duplicate(
        &self,
        source_id: &str,
        new_id: &str,
        new_name: &str,
    ) -> Result<Option<Profile>> {
        match self.get(source_id).await? {
            None => Ok(None),
            Some(source) => {
                let copy = Profile {
                    id: new_id.to_owned(),
                    name: new_name.to_owned(),
                    settings: source.settings.clone(),
                };
                self.save(&copy).await?;
                Ok(Some(copy))
            }
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

    fn store(dir: &TempDir) -> ProfileStore {
        ProfileStore::new(dir.path())
    }

    // Scenario: create and get a profile round-trips
    #[tokio::test]
    async fn create_and_get_round_trips() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let created = s.create("p1", "My Profile").await.unwrap();
        let loaded = s.get("p1").await.unwrap();

        assert_eq!(loaded, Some(created));
    }

    // Scenario: get returns None for absent profile
    #[tokio::test]
    async fn get_returns_none_for_absent_profile() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        assert_eq!(s.get("nope").await.unwrap(), None);
    }

    // Scenario: list returns all profiles sorted by id
    #[tokio::test]
    async fn list_returns_all_profiles_sorted_by_id() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.create("z-profile", "Z").await.unwrap();
        s.create("a-profile", "A").await.unwrap();
        s.create("m-profile", "M").await.unwrap();
        let profiles = s.list().await.unwrap();
        let ids: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();

        assert_eq!(ids, vec!["a-profile", "m-profile", "z-profile"]);
    }

    // Scenario: delete removes a profile; absent is a no-op
    #[tokio::test]
    async fn delete_removes_profile() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.create("p1", "P").await.unwrap();
        s.delete("p1").await.unwrap();

        assert_eq!(s.get("p1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn delete_absent_profile_is_ok() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        assert!(s.delete("never-existed").await.is_ok());
    }

    // Scenario: set_active and get_active round-trip
    #[tokio::test]
    async fn set_active_and_get_active_round_trip() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.create("p1", "Profile One").await.unwrap();
        s.set_active("p1").await.unwrap();
        let active = s.get_active().await.unwrap();

        assert_eq!(active.as_ref().map(|p| p.id.as_str()), Some("p1"));
    }

    // Scenario: active_id returns None before any active is set
    #[tokio::test]
    async fn active_id_returns_none_when_not_set() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        assert_eq!(s.active_id().await.unwrap(), None);
    }

    // Scenario: switching active profile changes get_active
    #[tokio::test]
    async fn switching_active_profile_changes_get_active() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.create("p1", "One").await.unwrap();
        s.create("p2", "Two").await.unwrap();
        s.set_active("p1").await.unwrap();
        s.set_active("p2").await.unwrap();
        let active = s.get_active().await.unwrap();

        assert_eq!(active.as_ref().map(|p| p.id.as_str()), Some("p2"));
    }

    // Scenario: save persists settings overrides
    #[tokio::test]
    async fn save_persists_settings_overrides() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let mut profile = s.create("p1", "P").await.unwrap();
        profile
            .settings
            .insert("theme".to_owned(), r#""dark""#.to_owned());
        s.save(&profile).await.unwrap();

        let loaded = s.get("p1").await.unwrap().unwrap();
        assert_eq!(
            loaded.settings.get("theme").map(String::as_str),
            Some(r#""dark""#)
        );
    }

    // Scenario: duplicate makes an independent copy
    #[tokio::test]
    async fn duplicate_makes_independent_copy() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        let mut original = s.create("orig", "Original").await.unwrap();
        original.settings.insert("k".to_owned(), "v".to_owned());
        s.save(&original).await.unwrap();

        let copy = s.duplicate("orig", "copy", "Copy").await.unwrap().unwrap();

        // Copy is independent: modifying original does not affect it.
        let mut updated = original.clone();
        updated
            .settings
            .insert("k".to_owned(), "changed".to_owned());
        s.save(&updated).await.unwrap();

        let reloaded_copy = s.get("copy").await.unwrap().unwrap();
        assert_eq!(
            reloaded_copy.settings.get("k").map(String::as_str),
            Some("v")
        );
        assert_eq!(copy.name, "Copy");
    }

    // Scenario: external edit is reflected on next read (no caching)
    #[tokio::test]
    async fn external_edit_is_reflected_on_next_read() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);

        s.create("p1", "Old Name").await.unwrap();

        let path = dir.path().join("config").join("profiles").join("p1.json");
        let raw = std::fs::read_to_string(&path).unwrap();
        let mut p: Profile = serde_json::from_str(&raw).unwrap();
        p.name = "New Name".to_owned();
        std::fs::write(&path, serde_json::to_string_pretty(&p).unwrap()).unwrap();

        let loaded = s.get("p1").await.unwrap().unwrap();
        assert_eq!(loaded.name, "New Name");
    }
}
