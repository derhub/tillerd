//! Tauri bridge for the orchestrator settings plane. Builds the `app/settings` CQS
//! command/query values and dispatches them through the managed `Bus<Ctx>`; the
//! renderer reaches it through the `@tillerd/sdk` settings client. Values cross the
//! IPC boundary as JSON values and are persisted as JSON strings (`value_json`).

use orchestrator::app::settings::{ApplySetting, GetSetting, ListSettings};
use orchestrator::entities::{ProjectId, SettingScope};
use orchestrator::shared::Bus;
use orchestrator::Ctx;
use serde::Serialize;
use serde_json::Value;
use tauri::State;

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

#[tauri::command]
pub async fn setting_get(
    scope: String,
    project_id: Option<String>,
    key: String,
    bus: State<'_, Bus<Ctx>>,
) -> Result<Option<Value>, String> {
    let scope = parse_scope(&scope, project_id)?;
    match bus
        .query(GetSetting { scope, key })
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
    let scope = parse_scope(&scope, project_id)?;
    let value_json = serde_json::to_string(&value).map_err(|e| e.to_string())?;
    bus.execute(ApplySetting {
        scope,
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
) -> Result<Vec<SettingEntryResponse>, String> {
    let scope = parse_scope(&scope, project_id)?;
    bus.query(ListSettings { scope })
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|e| {
            let value: Value =
                serde_json::from_str(&e.value_json).map_err(|err| err.to_string())?;
            Ok(SettingEntryResponse { key: e.key, value })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
