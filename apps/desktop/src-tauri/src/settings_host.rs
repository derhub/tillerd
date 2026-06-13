//! Tauri bridge for the orchestrator settings store. Delegates to the host-agnostic
//! `Store` settings API; the renderer reaches it through the `@tillerd/sdk` settings
//! client. Values cross the IPC boundary as JSON values and are persisted as JSON
//! strings (`value_json`) by the store.

use std::sync::Arc;

use orchestrator::persistence::{ProjectId, SettingScope, Store};
use serde::Serialize;
use serde_json::Value;
use tauri::State;

use crate::orchestrator_host::OrchestratorState;

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SettingEntryResponse {
    pub key: String,
    pub value: Value,
}

fn store_or_err(state: &OrchestratorState) -> Result<Arc<dyn Store>, String> {
    state
        .store_arc()
        .ok_or_else(|| "orchestrator not ready".to_string())
}

/// Map the wire scope (`"global"` / `"project"` + optional project id) to a `SettingScope`.
fn parse_scope(scope: &str, project_id: Option<String>) -> Result<SettingScope, String> {
    match scope {
        "global" => Ok(SettingScope::Global),
        "project" => project_id
            .map(|p| SettingScope::Project(ProjectId::new(p)))
            .ok_or_else(|| "project scope requires projectId".to_string()),
        other => Err(format!("unknown setting scope: {other}")),
    }
}

pub fn do_setting_get(
    store: &Arc<dyn Store>,
    scope: SettingScope,
    key: String,
) -> Result<Option<Value>, String> {
    match store
        .get_setting(&scope, &key)
        .map_err(|e| format!("{e:?}"))?
    {
        Some(s) => serde_json::from_str(&s)
            .map(Some)
            .map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

pub fn do_setting_set(
    store: &Arc<dyn Store>,
    scope: SettingScope,
    key: String,
    value: Value,
) -> Result<(), String> {
    let value_json = serde_json::to_string(&value).map_err(|e| e.to_string())?;
    store
        .set_setting(&scope, &key, &value_json)
        .map_err(|e| format!("{e:?}"))
}

pub fn do_setting_list(
    store: &Arc<dyn Store>,
    scope: SettingScope,
) -> Result<Vec<SettingEntryResponse>, String> {
    store
        .list_settings(&scope)
        .map_err(|e| format!("{e:?}"))?
        .into_iter()
        .map(|e| {
            let value: Value =
                serde_json::from_str(&e.value_json).map_err(|err| err.to_string())?;
            Ok(SettingEntryResponse { key: e.key, value })
        })
        .collect()
}

#[tauri::command]
pub fn setting_get(
    scope: String,
    project_id: Option<String>,
    key: String,
    state: State<'_, OrchestratorState>,
) -> Result<Option<Value>, String> {
    let store = store_or_err(&state)?;
    let scope = parse_scope(&scope, project_id)?;
    do_setting_get(&store, scope, key)
}

#[tauri::command]
pub fn setting_set(
    scope: String,
    project_id: Option<String>,
    key: String,
    value: Value,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let store = store_or_err(&state)?;
    let scope = parse_scope(&scope, project_id)?;
    do_setting_set(&store, scope, key, value)
}

#[tauri::command]
pub fn setting_list(
    scope: String,
    project_id: Option<String>,
    state: State<'_, OrchestratorState>,
) -> Result<Vec<SettingEntryResponse>, String> {
    let store = store_or_err(&state)?;
    let scope = parse_scope(&scope, project_id)?;
    do_setting_list(&store, scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use orchestrator::persistence::memory::InMemoryStore;

    fn fake_store() -> Arc<dyn Store> {
        Arc::new(InMemoryStore::new())
    }

    #[test]
    fn setting_round_trips_a_json_value() {
        let store = fake_store();
        do_setting_set(
            &store,
            SettingScope::Global,
            "theme".to_string(),
            serde_json::json!("dark"),
        )
        .unwrap();
        let got = do_setting_get(&store, SettingScope::Global, "theme".to_string()).unwrap();
        assert_eq!(got, Some(serde_json::json!("dark")));
    }

    #[test]
    fn unset_key_resolves_to_none() {
        let store = fake_store();
        let got = do_setting_get(&store, SettingScope::Global, "missing".to_string()).unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn list_returns_decoded_entries() {
        let store = fake_store();
        do_setting_set(
            &store,
            SettingScope::Global,
            "a".to_string(),
            serde_json::json!(1),
        )
        .unwrap();
        let listed = do_setting_list(&store, SettingScope::Global).unwrap();
        assert_eq!(
            listed,
            vec![SettingEntryResponse {
                key: "a".to_string(),
                value: serde_json::json!(1),
            }]
        );
    }

    #[test]
    fn entry_response_serializes_to_the_sdk_shape() {
        let entry = SettingEntryResponse {
            key: "k".to_string(),
            value: serde_json::json!(true),
        };
        let obj = serde_json::to_value(&entry).unwrap();
        let mut keys: Vec<&str> = obj
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["key", "value"]);
    }

    #[test]
    fn project_scope_requires_a_project_id() {
        let err = parse_scope("project", None).unwrap_err();
        assert!(err.contains("projectId"));
    }
}
