//! Tauri bridge for the orchestrator settings plane. Builds the `app/settings` CQS
//! command/query values and dispatches them through the managed `Bus<Ctx>`; the
//! renderer reaches it through the `@tillerd/sdk` settings client. Values cross the
//! IPC boundary as JSON values and are persisted as JSON strings (`value_json`).

use orchestrator::app::settings::{ApplySetting, GetSetting, ListSettings, SettingView};
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
