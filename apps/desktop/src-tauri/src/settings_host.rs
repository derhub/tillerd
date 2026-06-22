//! Tauri bridge for the orchestrator settings plane. Builds the `app/settings` CQS
//! command/query values and dispatches them through the managed `Bus<Ctx>`; the
//! renderer reaches it through the `@tillerd/sdk` settings client. Values cross the
//! IPC boundary as JSON values and are persisted as JSON strings (`value_json`).
//!
//! Hand-written shims live here when the wire encoding requires non-mechanical
//! conversion: `scope`+`projectId` -> `SettingScope` parse, JSON value <-> string.

use orchestrator::app::settings::{
    ApplySetting, GetSetting, ListSettings, ResetSetting, ResolveSetting, ResolveSettings,
    SettingView,
};
use orchestrator::shared::Bus;
use orchestrator::Ctx;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn setting_get(
    scope: String,
    project_id: Option<String>,
    key: String,
    bus: State<'_, Bus<Ctx>>,
) -> Result<Option<Value>, String> {
    match bus
        .query(GetSetting {
            scope,
            project_id,
            key,
        })
        .await
        .map_err(|e| e.to_string())?
    {
        Some(raw) => serde_json::from_str(&raw)
            .map(Some)
            .map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn setting_set(
    scope: String,
    project_id: Option<String>,
    key: String,
    value: Value,
    bus: State<'_, Bus<Ctx>>,
) -> Result<(), String> {
    let value_json = serde_json::to_string(&value).map_err(|e| e.to_string())?;
    bus.execute(ApplySetting {
        scope,
        project_id,
        key,
        value_json,
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn setting_list(
    scope: String,
    project_id: Option<String>,
    bus: State<'_, Bus<Ctx>>,
) -> Result<Vec<SettingView>, String> {
    bus.query(ListSettings { scope, project_id })
        .await
        .map_err(|e| e.to_string())
}

/// Clear a setting override at a scope (revert to inherited/default).
/// `scope` is `"global"` or `"project"`; `project_id` is required for the project scope.
#[tauri::command]
pub async fn setting_reset(
    scope: String,
    project_id: Option<String>,
    key: String,
    bus: State<'_, Bus<Ctx>>,
) -> Result<(), String> {
    bus.execute(ResetSetting {
        scope,
        project_id,
        key,
    })
    .await
    .map_err(|e| e.to_string())
}

/// Effective value for a project: project override if set, else global, else `None`.
/// Returns the raw JSON-encoded string (not parsed), matching the bus output.
#[tauri::command]
pub async fn setting_resolve(
    project_id: String,
    key: String,
    bus: State<'_, Bus<Ctx>>,
) -> Result<Option<String>, String> {
    bus.query(ResolveSetting { project_id, key })
        .await
        .map_err(|e| e.to_string())
}

/// Full effective settings map for a project (global merged with project overrides).
/// Values in each `SettingView` are parsed back to JSON.
#[tauri::command]
pub async fn settings_resolve(
    project_id: String,
    bus: State<'_, Bus<Ctx>>,
) -> Result<Vec<SettingView>, String> {
    bus.query(ResolveSettings { project_id })
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_view_serializes_to_the_sdk_shape() {
        let entry = SettingView {
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
}
