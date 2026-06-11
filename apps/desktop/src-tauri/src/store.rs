use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;
use tokio::sync::Mutex;

use tillerd_paths::runtime_dir;

/// Native app-data store: user preferences plus the session registry (sessionId -> cwd). Replaces
/// the server-side sqlite registry on the desktop path (design D6). Persisted as JSON.
#[derive(Default, Serialize, Deserialize)]
struct StoreData {
    #[serde(default)]
    prefs: HashMap<String, Value>,
    #[serde(default)]
    registry: HashMap<String, String>,
}

pub struct StoreState {
    inner: Mutex<StoreData>,
}

impl StoreState {
    pub fn load() -> Self {
        let data = std::fs::read(store_path())
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        StoreState {
            inner: Mutex::new(data),
        }
    }
}

fn store_path() -> PathBuf {
    runtime_dir().join("desktop-store.json")
}

async fn persist(data: &StoreData) {
    let Ok(bytes) = serde_json::to_vec_pretty(data) else {
        return;
    };
    let path = store_path();
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let _ = tokio::fs::write(path, bytes).await;
}

#[derive(Serialize)]
pub struct RegistryEntry {
    #[serde(rename = "sessionId")]
    session_id: String,
    cwd: String,
}

#[tauri::command]
pub async fn pref_get(key: String, state: State<'_, StoreState>) -> Result<Option<Value>, ()> {
    Ok(state.inner.lock().await.prefs.get(&key).cloned())
}

#[tauri::command]
pub async fn pref_set(key: String, value: Value, state: State<'_, StoreState>) -> Result<(), ()> {
    let mut data = state.inner.lock().await;
    data.prefs.insert(key, value);
    persist(&data).await;
    Ok(())
}

#[tauri::command]
pub async fn registry_get(
    session_id: String,
    state: State<'_, StoreState>,
) -> Result<Option<String>, ()> {
    Ok(state.inner.lock().await.registry.get(&session_id).cloned())
}

#[tauri::command]
pub async fn registry_set(
    session_id: String,
    cwd: String,
    state: State<'_, StoreState>,
) -> Result<(), ()> {
    let mut data = state.inner.lock().await;
    data.registry.insert(session_id, cwd);
    persist(&data).await;
    Ok(())
}

#[tauri::command]
pub async fn registry_remove(session_id: String, state: State<'_, StoreState>) -> Result<(), ()> {
    let mut data = state.inner.lock().await;
    data.registry.remove(&session_id);
    persist(&data).await;
    Ok(())
}

#[tauri::command]
pub async fn registry_list(state: State<'_, StoreState>) -> Result<Vec<RegistryEntry>, ()> {
    Ok(state
        .inner
        .lock()
        .await
        .registry
        .iter()
        .map(|(session_id, cwd)| RegistryEntry {
            session_id: session_id.clone(),
            cwd: cwd.clone(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn store_data_default_prefs_are_empty() {
        assert!(StoreData::default().prefs.is_empty());
    }

    #[test]
    fn store_data_default_registry_is_empty() {
        assert!(StoreData::default().registry.is_empty());
    }

    #[test]
    fn store_data_pref_round_trips_json() {
        let mut data = StoreData::default();
        data.prefs.insert("theme".into(), serde_json::json!("dark"));
        let loaded: StoreData =
            serde_json::from_slice(&serde_json::to_vec(&data).unwrap()).unwrap();
        assert_eq!(loaded.prefs.get("theme"), Some(&serde_json::json!("dark")));
    }

    #[test]
    fn store_data_registry_round_trips_json() {
        let mut data = StoreData::default();
        data.registry
            .insert("sess-1".into(), "/home/user/project".into());
        let loaded: StoreData =
            serde_json::from_slice(&serde_json::to_vec(&data).unwrap()).unwrap();
        assert_eq!(
            loaded.registry.get("sess-1").map(String::as_str),
            Some("/home/user/project")
        );
    }

    #[test]
    fn store_data_tolerates_unknown_json_fields() {
        let json = br#"{"prefs":{},"registry":{},"unknown_field":"ignored"}"#;
        let data: StoreData = serde_json::from_slice(json).unwrap();
        assert!(data.prefs.is_empty());
    }

    #[test]
    #[serial]
    fn store_state_load_returns_default_when_file_absent() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("TILLERD_DIR", tmp.path());
        let state = StoreState::load();
        std::env::remove_var("TILLERD_DIR");

        let data = state.inner.blocking_lock();
        assert!(data.prefs.is_empty());
        assert!(data.registry.is_empty());
    }

    #[test]
    #[serial]
    fn store_state_load_reads_persisted_data() {
        let tmp = tempfile::tempdir().unwrap();
        let store_file = tmp.path().join("desktop-store.json");
        std::fs::write(
            &store_file,
            br#"{"prefs":{"lang":"en"},"registry":{"sess-abc":"/workspace"}}"#,
        )
        .unwrap();

        std::env::set_var("TILLERD_DIR", tmp.path());
        let state = StoreState::load();
        std::env::remove_var("TILLERD_DIR");

        let data = state.inner.blocking_lock();
        assert_eq!(data.prefs.get("lang"), Some(&serde_json::json!("en")));
        assert_eq!(
            data.registry.get("sess-abc").map(String::as_str),
            Some("/workspace")
        );
    }

    #[test]
    #[serial]
    fn store_state_load_returns_default_for_corrupt_file() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("desktop-store.json"), b"not json").unwrap();

        std::env::set_var("TILLERD_DIR", tmp.path());
        let state = StoreState::load();
        std::env::remove_var("TILLERD_DIR");

        let data = state.inner.blocking_lock();
        assert!(data.prefs.is_empty());
    }
}
