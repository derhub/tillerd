use std::fs;
use std::path::Path;

use crate::error::{OrchestratorError, Result};

pub const HOOK_MARKER: &str = "tillerd-notify";

const SETTINGS_FILE: &str = "settings.json";

const HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PostToolUse",
    "PermissionRequest",
    "Stop",
    "SessionEnd",
];

fn settings_path(agent_home: &Path) -> std::path::PathBuf {
    agent_home.join(SETTINGS_FILE)
}

fn backup_path(agent_home: &Path) -> std::path::PathBuf {
    agent_home.join("settings.json.bak")
}

fn matcher_for(event: &str) -> &'static str {
    if event == "PostToolUse" {
        "*"
    } else {
        ""
    }
}

fn read_settings(path: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str(&raw).unwrap_or(serde_json::Value::Object(Default::default()))
}

fn is_tillerd_entry(entry: &serde_json::Value) -> bool {
    entry["hooks"]
        .as_array()
        .map(|hooks| {
            hooks.iter().any(|h| {
                h["command"]
                    .as_str()
                    .map(|c| c.contains(HOOK_MARKER))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn is_legacy_entry(entry: &serde_json::Value) -> bool {
    entry["hooks"]
        .as_array()
        .map(|hooks| {
            hooks.iter().any(|h| {
                h["command"]
                    .as_str()
                    .map(|c| c.contains("curl"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn persist(agent_home: &Path, settings: &serde_json::Value) -> Result<()> {
    let path = settings_path(agent_home);
    let bak = backup_path(agent_home);

    // Backup: overwrite single .bak file.
    if path.exists() {
        fs::copy(&path, &bak).map_err(|e| io_err("backup settings", e))?;
    }

    // Atomic write via temp file + rename.
    let tmp = path.with_extension("json.tmp");
    let serialized =
        serde_json::to_string_pretty(settings).map_err(|e| io_err("serialize settings", e))?;
    fs::write(&tmp, format!("{serialized}\n")).map_err(|e| io_err("write settings.tmp", e))?;
    fs::rename(&tmp, &path).map_err(|e| io_err("rename settings.tmp", e))?;
    Ok(())
}

fn io_err(ctx: &str, e: impl std::fmt::Display) -> OrchestratorError {
    OrchestratorError::Surface {
        surface: "agent-setup".to_string(),
        reason: format!("{ctx}: {e}"),
    }
}

/// Installs the tillerd notify hooks into `agent_home/settings.json`.
/// Idempotent: no-op when hooks are already present and up to date.
pub fn install(agent_home: &Path, notify_command: &str) -> Result<()> {
    let path = settings_path(agent_home);
    let mut settings = read_settings(&path);

    let mut hooks_map = settings["hooks"].as_object().cloned().unwrap_or_default();

    let mut changed = false;
    for &event in HOOK_EVENTS {
        let list = hooks_map
            .get(event)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let already_installed = list
            .iter()
            .filter(|e| is_tillerd_entry(e))
            .any(|e| !is_legacy_entry(e));

        if already_installed {
            continue;
        }

        // Remove any legacy (curl-based) tillerd entries, then append the current one.
        let mut cleaned: Vec<serde_json::Value> =
            list.into_iter().filter(|e| !is_tillerd_entry(e)).collect();
        cleaned.push(serde_json::json!({
            "matcher": matcher_for(event),
            "hooks": [{ "type": "command", "command": notify_command }]
        }));
        hooks_map.insert(event.to_string(), serde_json::Value::Array(cleaned));
        changed = true;
    }

    if !changed {
        return Ok(());
    }

    settings["hooks"] = serde_json::Value::Object(hooks_map);
    persist(agent_home, &settings)
}

/// Removes all tillerd notify hooks from `agent_home/settings.json`.
/// Idempotent: no-op when no hooks are present.
pub fn uninstall(agent_home: &Path) -> Result<()> {
    let path = settings_path(agent_home);
    let mut settings = read_settings(&path);

    let Some(hooks_obj) = settings["hooks"].as_object_mut() else {
        return Ok(());
    };

    let mut changed = false;
    for event in HOOK_EVENTS {
        let Some(list) = hooks_obj.get(*event).and_then(|v| v.as_array()).cloned() else {
            continue;
        };
        let filtered: Vec<serde_json::Value> = list
            .iter()
            .filter(|e| !is_tillerd_entry(e))
            .cloned()
            .collect();
        if filtered.len() != list.len() {
            hooks_obj.insert((*event).to_string(), serde_json::Value::Array(filtered));
            changed = true;
        }
    }

    if !changed {
        return Ok(());
    }

    persist(agent_home, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write_settings(dir: &std::path::Path, value: serde_json::Value) {
        let path = dir.join(SETTINGS_FILE);
        fs::write(path, serde_json::to_string_pretty(&value).unwrap() + "\n").unwrap();
    }

    fn read_settings_value(dir: &std::path::Path) -> serde_json::Value {
        read_settings(&dir.join(SETTINGS_FILE))
    }

    const NOTIFY_CMD: &str = "tillerd-notify --gate /tmp/gate.sock";

    #[test]
    fn install_adds_hooks_for_all_six_events() {
        let dir = tempfile::tempdir().unwrap();
        install(dir.path(), NOTIFY_CMD).unwrap();
        let s = read_settings_value(dir.path());
        for event in HOOK_EVENTS {
            assert!(
                s["hooks"][event].as_array().is_some(),
                "missing hook for {event}"
            );
        }
    }

    #[test]
    fn install_is_idempotent_no_change_on_second_call() {
        let dir = tempfile::tempdir().unwrap();
        install(dir.path(), NOTIFY_CMD).unwrap();
        let s1 = read_settings_value(dir.path());
        install(dir.path(), NOTIFY_CMD).unwrap();
        let s2 = read_settings_value(dir.path());
        assert_eq!(s1, s2);
    }

    #[test]
    fn install_preserves_user_owned_hooks() {
        let dir = tempfile::tempdir().unwrap();
        write_settings(
            dir.path(),
            json!({
                "hooks": {
                    "SessionStart": [
                        { "matcher": "", "hooks": [{ "type": "command", "command": "my-hook" }] }
                    ]
                }
            }),
        );
        install(dir.path(), NOTIFY_CMD).unwrap();
        let s = read_settings_value(dir.path());
        let list = s["hooks"]["SessionStart"].as_array().unwrap();
        assert!(
            list.iter().any(|e| e["hooks"][0]["command"] == "my-hook"),
            "user hook removed"
        );
        assert!(list.iter().any(is_tillerd_entry), "tillerd hook missing");
    }

    #[test]
    fn install_migrates_legacy_curl_entry() {
        let dir = tempfile::tempdir().unwrap();
        write_settings(
            dir.path(),
            json!({
                "hooks": {
                    "Stop": [
                        {
                            "matcher": "",
                            "hooks": [{ "type": "command", "command": "curl http://localhost:5000/tillerd-notify" }]
                        }
                    ]
                }
            }),
        );
        install(dir.path(), NOTIFY_CMD).unwrap();
        let s = read_settings_value(dir.path());
        let list = s["hooks"]["Stop"].as_array().unwrap();
        // Legacy curl entry must be replaced — no curl remaining.
        assert!(
            !list.iter().any(is_legacy_entry),
            "legacy curl entry not removed"
        );
        assert!(
            list.iter().any(is_tillerd_entry),
            "notify hook not installed"
        );
    }

    #[test]
    fn uninstall_removes_tillerd_hooks_and_restores_user_hooks() {
        let dir = tempfile::tempdir().unwrap();
        write_settings(
            dir.path(),
            json!({
                "hooks": {
                    "Stop": [
                        { "matcher": "", "hooks": [{ "type": "command", "command": "my-hook" }] },
                        { "matcher": "", "hooks": [{ "type": "command", "command": NOTIFY_CMD }] }
                    ]
                }
            }),
        );
        uninstall(dir.path()).unwrap();
        let s = read_settings_value(dir.path());
        let list = s["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["hooks"][0]["command"], "my-hook");
    }

    #[test]
    fn uninstall_when_no_hooks_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        write_settings(dir.path(), json!({ "theme": "dark" }));
        uninstall(dir.path()).unwrap();
        let s = read_settings_value(dir.path());
        assert_eq!(s["theme"], "dark");
        assert!(s["hooks"].is_null());
    }

    #[test]
    fn install_creates_backup_file() {
        let dir = tempfile::tempdir().unwrap();
        write_settings(dir.path(), json!({}));
        install(dir.path(), NOTIFY_CMD).unwrap();
        assert!(dir.path().join("settings.json.bak").exists());
    }

    #[test]
    fn backup_is_overwritten_on_second_install() {
        let dir = tempfile::tempdir().unwrap();
        write_settings(dir.path(), json!({ "v": 1 }));
        install(dir.path(), NOTIFY_CMD).unwrap();
        let bak1 = fs::read_to_string(dir.path().join("settings.json.bak")).unwrap();

        // Modify settings and re-install to trigger another backup.
        write_settings(dir.path(), json!({ "v": 2 }));
        install(dir.path(), NOTIFY_CMD).unwrap();
        let bak2 = fs::read_to_string(dir.path().join("settings.json.bak")).unwrap();

        // The second backup should differ (it captured the v:2 file).
        assert_ne!(bak1, bak2);
    }
}
