//! Runtime contract test for every desktop IPC command. The app is built with the real context
//! (`crate::app_context()` — config + embedded frontend + resolved ACL) on the `tauri::test` mock
//! runtime, then each command registered in `run()` is invoked through the live IPC path with the
//! argument shape `@tillerd/sdk` + the desktop bridge actually send. A command that is missing from
//! the handler fails with "Command <name> not found"; a command whose argument struct drifts from
//! its body fails with "invalid args `<field>`". Either is a contract break the assertions catch —
//! at unit-test speed, before a full desktop e2e cycle. Business errors (no store, no daemon) are
//! expected and ignored: they prove the command was reached.
//!
//! The invoke origin is `tauri://localhost` (local): app commands carry no ACL manifest, so the
//! authority's command check is skipped for a local origin and the command dispatches as it does in
//! the running app. Per-response key shapes are checked by `workspace_host`'s serde tests.

use crate::orchestrator_host::OrchestratorState;
use crate::surface_host::SurfaceState;
use crate::{bridge, diag, files, store, supervisor, surface_host, workspace_host};
use orchestrator::persistence::memory::InMemoryStore;
use orchestrator::persistence::{Store, SurfaceId};
use orchestrator::surface::{SurfaceApi, SurfaceEventSink};
use serial_test::serial;
use std::sync::Arc;
use tauri::test::{mock_builder, MockRuntime};
use tauri::Manager;
use tauri::WebviewWindow;

struct NullSink;
impl SurfaceEventSink for NullSink {
    fn on_bytes(&self, _: &SurfaceId, _: &[u8]) {}
    fn on_status(&self, _: &SurfaceId, _: &str) {}
    fn on_exit(&self, _: &SurfaceId, _: &str) {}
}

/// The app with the full handler set and every managed state the commands resolve, on the real
/// context so the live IPC path runs.
fn contract_app() -> tauri::App<MockRuntime> {
    let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
    let api = Arc::new(SurfaceApi::new(
        store,
        Arc::new(NullSink),
        "/tmp/tillerd-contract.sock".into(),
    ));
    let surfaces = SurfaceState {
        api,
        channels: Default::default(),
    };

    mock_builder()
        .manage(bridge::BridgeState::default())
        .manage(store::StoreState::load())
        .manage(supervisor::SupervisorState::default())
        .manage(OrchestratorState::default())
        .manage(surfaces)
        .invoke_handler(tauri::generate_handler![
            // `daemon_connect` is omitted: it takes a concrete `AppHandle` (= `AppHandle<Wry>`),
            // which `tauri::test`'s `MockRuntime` handler cannot register. Its only data argument is
            // an IPC `Channel`, so there is no plain-data arg shape to drift.
            bridge::daemon_send,
            bridge::daemon_disconnect,
            files::file_size,
            files::file_read,
            diag::log_forward,
            store::pref_get,
            store::pref_set,
            store::registry_get,
            store::registry_set,
            store::registry_remove,
            store::registry_list,
            supervisor::daemon_ensure,
            crate::orchestrator_host::orchestrator_status,
            crate::orchestrator_host::service_health,
            surface_host::surface_create,
            surface_host::surface_spawn,
            surface_host::surface_close,
            surface_host::surface_input,
            surface_host::surface_resize,
            surface_host::surface_detach,
            workspace_host::project_create,
            workspace_host::project_list,
            workspace_host::project_rename,
            workspace_host::project_archive,
            workspace_host::session_list,
            workspace_host::session_create,
            workspace_host::session_rename,
            workspace_host::session_archive,
            workspace_host::session_layout_set,
            workspace_host::session_layout_get,
            workspace_host::command_list,
            workspace_host::command_create,
            workspace_host::command_get,
            workspace_host::command_delete,
        ])
        .build(crate::app_context())
        .expect("app builds with the full command set + managed state")
}

fn main_webview(app: &tauri::App<MockRuntime>) -> WebviewWindow<MockRuntime> {
    match app.get_webview_window("main") {
        Some(w) => w,
        None => tauri::WebviewWindowBuilder::new(app, "main", Default::default())
            .build()
            .expect("main webview builds"),
    }
}

fn invoke(
    webview: &WebviewWindow<MockRuntime>,
    cmd: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    tauri::test::get_ipc_response(
        webview,
        tauri::webview::InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            // Local origin: app commands have no ACL manifest, so the authority check is skipped
            // for `tauri://localhost`, matching how the running webview reaches these commands.
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        },
    )
    .map(|b| b.deserialize::<serde_json::Value>().unwrap())
}

#[test]
#[serial] // mutates TILLERD_* env; serialized against the other env-sensitive command tests
fn every_desktop_ipc_command_is_registered_and_accepts_its_arg_shape() {
    // Hermetic runtime dir: store/registry writes and the daemon manifest read stay in a temp
    // dir, and `daemon_ensure` resolves the daemon binary to `/dev/null` (exists, so it is
    // chosen) whose `execve` fails instantly — proving the command is reached without spawning a
    // real daemon or waiting on reachability.
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var(tillerd_paths::ENV_TILLERD_DIR, tmp.path());
    std::env::set_var(tillerd_paths::ENV_DAEMON_BIN, "/dev/null");

    let app = contract_app();
    let webview = main_webview(&app);

    // A channel arg is sent by the renderer as the string `__CHANNEL__:<id>`.
    let channel = serde_json::Value::String("__CHANNEL__:1".into());

    // Each body mirrors what `@tillerd/sdk` + the desktop bridge send for that command. Optional
    // fields are populated so a rename to a required field is caught; required fields must be
    // present or deserialization fails (which is exactly what this test asserts against).
    let cases: Vec<(&str, serde_json::Value)> = vec![
        // `daemon_connect` excluded — see the handler list (concrete `AppHandle`, channel-only).
        ("daemon_send", serde_json::json!({ "bytes": [0u8, 1, 2] })),
        ("daemon_disconnect", serde_json::json!({})),
        (
            "file_size",
            serde_json::json!({ "path": "/no/such/contract/file" }),
        ),
        (
            "file_read",
            serde_json::json!({ "path": "/no/such/contract/file", "offset": 0, "length": 16 }),
        ),
        (
            "log_forward",
            serde_json::json!({ "level": "info", "msg": "contract", "extra": null }),
        ),
        ("pref_get", serde_json::json!({ "key": "contract" })),
        (
            "pref_set",
            serde_json::json!({ "key": "contract", "value": 1 }),
        ),
        (
            "registry_get",
            serde_json::json!({ "sessionId": "contract" }),
        ),
        (
            "registry_set",
            serde_json::json!({ "sessionId": "contract", "cwd": "/tmp" }),
        ),
        (
            "registry_remove",
            serde_json::json!({ "sessionId": "contract" }),
        ),
        ("registry_list", serde_json::json!({})),
        ("daemon_ensure", serde_json::json!({})),
        ("orchestrator_status", serde_json::json!({})),
        ("service_health", serde_json::json!({})),
        (
            "surface_create",
            serde_json::json!({ "channel": channel, "sessionId": "contract", "placement": "p", "cols": 80, "rows": 24, "cwd": null }),
        ),
        (
            "surface_spawn",
            serde_json::json!({ "sessionId": "contract" }),
        ),
        (
            "surface_close",
            serde_json::json!({ "sessionId": "contract", "placement": "p" }),
        ),
        (
            "surface_input",
            serde_json::json!({ "surfaceId": "contract", "bytes": [1u8] }),
        ),
        (
            "surface_resize",
            serde_json::json!({ "surfaceId": "contract", "cols": 80, "rows": 24 }),
        ),
        (
            "surface_detach",
            serde_json::json!({ "surfaceId": "contract" }),
        ),
        ("project_create", serde_json::json!({ "name": "contract" })),
        ("project_list", serde_json::json!({})),
        (
            "project_rename",
            serde_json::json!({ "id": "contract", "name": "x" }),
        ),
        ("project_archive", serde_json::json!({ "id": "contract" })),
        ("session_list", serde_json::json!({ "projectId": null })),
        (
            "session_create",
            serde_json::json!({ "projectId": null, "title": "x", "titleSource": "agentTitle", "templateId": null }),
        ),
        (
            "session_rename",
            serde_json::json!({ "id": "contract", "title": "x" }),
        ),
        ("session_archive", serde_json::json!({ "id": "contract" })),
        (
            "session_layout_set",
            serde_json::json!({ "id": "contract", "layoutJson": "{}" }),
        ),
        (
            "session_layout_get",
            serde_json::json!({ "id": "contract" }),
        ),
        ("command_list", serde_json::json!({})),
        (
            "command_create",
            serde_json::json!({ "req": { "name": "x", "cli": "/x", "args": [], "env": {} } }),
        ),
        ("command_get", serde_json::json!({ "id": "contract" })),
        ("command_delete", serde_json::json!({ "id": "contract" })),
    ];

    for (cmd, body) in cases {
        if let Err(value) = invoke(&webview, cmd, body) {
            let msg = value.to_string();
            assert!(
                !msg.contains(&format!("Command {cmd} not found")),
                "`{cmd}` is not registered in the invoke handler: {msg}"
            );
            assert!(
                !msg.contains("invalid args `"),
                "`{cmd}` rejected its documented argument shape (arg-shape drift): {msg}"
            );
            assert!(
                !msg.contains("not allowed"),
                "`{cmd}` was denied by the ACL: {msg}"
            );
        }
    }

    std::env::remove_var(tillerd_paths::ENV_TILLERD_DIR);
    std::env::remove_var(tillerd_paths::ENV_DAEMON_BIN);
}
