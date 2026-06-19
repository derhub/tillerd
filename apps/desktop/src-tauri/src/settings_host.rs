//! Tauri bridge for the orchestrator settings store. Delegates to the host-agnostic
//! `Settings` store; the renderer reaches it through the `@tillerd/sdk` settings client.
//! Values cross the IPC boundary as JSON values and are persisted as JSON strings
//! (`value_json`) by the store.

use orchestrator::entities::{ProjectId, SettingScope};
use orchestrator::store::Settings;
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

pub async fn do_setting_get(
    settings: &Settings,
    scope: SettingScope,
    key: String,
) -> Result<Option<Value>, String> {
    match settings
        .get(scope, key)
        .await
        .map_err(|e| format!("{e:?}"))?
    {
        Some(s) => serde_json::from_str(&s)
            .map(Some)
            .map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

pub async fn do_setting_set(
    settings: &Settings,
    scope: SettingScope,
    key: String,
    value: Value,
) -> Result<(), String> {
    let value_json = serde_json::to_string(&value).map_err(|e| e.to_string())?;
    settings
        .set(scope, key, value_json)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_setting_list(
    settings: &Settings,
    scope: SettingScope,
) -> Result<Vec<SettingEntryResponse>, String> {
    settings
        .list(scope)
        .await
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
pub async fn setting_get(
    scope: String,
    project_id: Option<String>,
    key: String,
    state: State<'_, OrchestratorState>,
) -> Result<Option<Value>, String> {
    let storage = state
        .storage()
        .ok_or_else(|| "orchestrator not ready".to_string())?;
    let scope = parse_scope(&scope, project_id)?;
    do_setting_get(&storage.settings, scope, key).await
}

#[tauri::command]
pub async fn setting_set(
    scope: String,
    project_id: Option<String>,
    key: String,
    value: Value,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let storage = state
        .storage()
        .ok_or_else(|| "orchestrator not ready".to_string())?;
    let scope = parse_scope(&scope, project_id)?;
    do_setting_set(&storage.settings, scope, key, value).await
}

#[tauri::command]
pub async fn setting_list(
    scope: String,
    project_id: Option<String>,
    state: State<'_, OrchestratorState>,
) -> Result<Vec<SettingEntryResponse>, String> {
    let storage = state
        .storage()
        .ok_or_else(|| "orchestrator not ready".to_string())?;
    let scope = parse_scope(&scope, project_id)?;
    do_setting_list(&storage.settings, scope).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use orchestrator::infra::memory::MemoryBackend;
    use orchestrator::store::Storage;

    fn fake_settings() -> Settings {
        Storage::in_memory(MemoryBackend::new()).settings
    }

    #[tokio::test]
    async fn setting_round_trips_a_json_value() {
        let settings = fake_settings();
        do_setting_set(
            &settings,
            SettingScope::Global,
            "theme".to_string(),
            serde_json::json!("dark"),
        )
        .await
        .unwrap();
        let got = do_setting_get(&settings, SettingScope::Global, "theme".to_string())
            .await
            .unwrap();
        assert_eq!(got, Some(serde_json::json!("dark")));
    }

    #[tokio::test]
    async fn unset_key_resolves_to_none() {
        let settings = fake_settings();
        let got = do_setting_get(&settings, SettingScope::Global, "missing".to_string())
            .await
            .unwrap();
        assert_eq!(got, None);
    }

    #[tokio::test]
    async fn list_returns_decoded_entries() {
        let settings = fake_settings();
        do_setting_set(
            &settings,
            SettingScope::Global,
            "a".to_string(),
            serde_json::json!(1),
        )
        .await
        .unwrap();
        let listed = do_setting_list(&settings, SettingScope::Global)
            .await
            .unwrap();
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
