//! Runtime contract test for every desktop IPC command. The app is built with the real context
//! (`crate::app_context()` -- config + embedded frontend + resolved ACL) on the `tauri::test` mock
//! runtime over a `:memory:` `Ctx` (migrations applied, `FakeRuntime`, `SqliteKv`), then each
//! command registered in `run()` is invoked through the live IPC path with the argument shape
//! `@tillerd/client-bindings` actually sends. A command that is missing from the handler
//! fails with "Command <name> not found"; a command whose argument struct drifts from its body
//! fails with "invalid args `<field>`". Either is a contract break the assertions catch -- at
//! unit-test speed, before a full desktop e2e cycle. Business errors (no daemon, store not ready)
//! are expected and ignored: they prove the command was reached.
//!
//! The invoke origin is `tauri://localhost` (local): app commands carry no ACL manifest, so the
//! authority's command check is skipped for a local origin and the command dispatches as it does in
//! the running app. Per-response key shapes are checked by `workspace_host`'s serde tests.

use orchestrator::context::Ctx;
use orchestrator::shared::bus::Bus;
use serial_test::serial;
use tauri::test::{mock_builder, MockRuntime};
use tauri::{Manager, WebviewWindow};

use crate::orchestrator_host::OrchestratorState;
use crate::transport::macros::collect_transport;

/// Build a `:memory:` `Ctx` with migrations applied and a `FakeRuntime`, via the
/// orchestrator's app-owned test edge. This is the context `Bus<Ctx>` dispatches over;
/// the contract test drives every command through the live IPC path against it.
async fn memory_ctx() -> Ctx {
    orchestrator::boot::test_ctx()
        .await
        .expect("in-memory test Ctx")
}

/// The app with the full handler set and every managed state the commands resolve, on the real
/// context (real `generate_context!()`, resolved ACL) so the live IPC path runs.
fn contract_app() -> tauri::App<MockRuntime> {
    // Build a `:memory:` Ctx synchronously from a one-shot tokio runtime.
    let ctx = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(memory_ctx());

    let bus = Bus::new(ctx);

    mock_builder()
        .manage(bus)
        .manage(OrchestratorState::default())
        .manage(crate::menu::LeaderMenuState::default())
        .invoke_handler(collect_transport!())
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
    // Hermetic runtime dir: command side effects stay in a temp dir.
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var(tillerd_paths::ENV_TILLERD_DIR, tmp.path());
    std::env::set_var(tillerd_paths::ENV_DAEMON_BIN, "/dev/null");

    let app = contract_app();
    let webview = main_webview(&app);

    // A channel arg is sent by the renderer as the string `__CHANNEL__:<id>`.
    let channel = serde_json::Value::String("__CHANNEL__:1".into());

    // Each body mirrors what `@tillerd/client-bindings` sends for that command. Optional
    // fields are populated so a rename to a required field is caught; required fields must be
    // present or deserialization fails (which is exactly what this test asserts against).
    let cases: Vec<(&str, serde_json::Value)> = vec![
        ("orchestrator_status", serde_json::json!({})),
        ("service_health", serde_json::json!({})),
        (
            "surface_resolve_or_spawn",
            serde_json::json!({ "session": "contract", "placement": "p", "cwd": null, "cols": 80, "rows": 24 }),
        ),
        (
            "surface_channel",
            serde_json::json!({ "channel": channel, "req": { "surfaceId": "contract" } }),
        ),
        (
            "surface_channel_send",
            serde_json::json!({ "key": "contract", "msg": { "kind": "input", "bytes": [1u8] } }),
        ),
        (
            "surface_channel_close",
            serde_json::json!({ "req": { "surfaceId": "contract" } }),
        ),
        (
            "log_channel",
            serde_json::json!({ "channel": channel, "req": { "service": "contract" } }),
        ),
        (
            "log_channel_close",
            serde_json::json!({ "req": { "service": "contract" } }),
        ),
        (
            "logs_changed_channel",
            serde_json::json!({ "channel": channel, "req": { "channelId": "contract" } }),
        ),
        (
            "logs_changed_channel_close",
            serde_json::json!({ "req": { "channelId": "contract" } }),
        ),
        (
            "notification_channel",
            serde_json::json!({ "channel": channel, "req": { "channelId": "contract" } }),
        ),
        (
            "surface_status_channel",
            serde_json::json!({ "channel": channel, "req": { "channelId": "contract" } }),
        ),
        (
            "surface_status_channel_close",
            serde_json::json!({ "req": { "channelId": "contract" } }),
        ),
        (
            "notification_channel_close",
            serde_json::json!({ "req": { "channelId": "contract" } }),
        ),
        ("log_list", serde_json::json!({})),
        (
            "log_tail",
            serde_json::json!({ "path": "/dev/null", "from": 0, "maxBytes": 1, "align": false }),
        ),
        (
            "surface_spawn",
            serde_json::json!({ "sessionId": "contract" }),
        ),
        ("surface_close", serde_json::json!({ "id": "contract" })),
        ("surface_detach", serde_json::json!({ "id": "contract" })),
        (
            "window_open",
            serde_json::json!({ "label": "detached-contract", "query": "?w=detached&session=s&placement=p" }),
        ),
        ("window_focus", serde_json::json!({ "label": "main" })),
        ("window_close", serde_json::json!({ "label": "main" })),
        ("project_create", serde_json::json!({ "name": "contract" })),
        ("project_list", serde_json::json!({})),
        (
            "project_rename",
            serde_json::json!({ "id": "contract", "name": "x" }),
        ),
        ("project_archive", serde_json::json!({ "id": "contract" })),
        ("project_delete", serde_json::json!({ "id": "contract" })),
        (
            "project_reorder",
            serde_json::json!({ "id": "contract", "sortOrder": 0 }),
        ),
        (
            "project_move",
            serde_json::json!({ "id": "contract", "workspaceId": "contract" }),
        ),
        ("project_get", serde_json::json!({ "id": "contract" })),
        (
            "project_search",
            serde_json::json!({ "workspaceId": "contract", "query": "x", "limit": 10 }),
        ),
        ("project_restore", serde_json::json!({ "id": "contract" })),
        (
            "project_duplicate",
            serde_json::json!({ "sourceId": "contract", "name": "copy" }),
        ),
        ("project_pin", serde_json::json!({ "id": "contract" })),
        ("project_unpin", serde_json::json!({ "id": "contract" })),
        (
            "project_stop_surfaces",
            serde_json::json!({ "id": "contract" }),
        ),
        (
            "workspace_create",
            serde_json::json!({ "name": "contract" }),
        ),
        ("workspace_list", serde_json::json!({})),
        ("workspace_activity", serde_json::json!({})),
        (
            "workspace_rename",
            serde_json::json!({ "id": "contract", "name": "x" }),
        ),
        (
            "workspace_reorder",
            serde_json::json!({ "id": "contract", "sortOrder": 0 }),
        ),
        ("workspace_delete", serde_json::json!({ "id": "contract" })),
        ("workspace_get", serde_json::json!({ "id": "contract" })),
        ("workspace_archive", serde_json::json!({ "id": "contract" })),
        ("workspace_restore", serde_json::json!({ "id": "contract" })),
        ("workspace_pin", serde_json::json!({ "id": "contract" })),
        ("workspace_unpin", serde_json::json!({ "id": "contract" })),
        (
            "workspace_stop_surfaces",
            serde_json::json!({ "id": "contract" }),
        ),
        ("surface_get", serde_json::json!({ "id": "contract" })),
        (
            "surface_list_by_session",
            serde_json::json!({ "session": "contract", "limit": null, "offset": null, "after": null }),
        ),
        ("surface_list_resumable", serde_json::json!({})),
        (
            "surface_find_by_placement",
            serde_json::json!({ "session": "contract", "placement": "main" }),
        ),
        ("surface_stop", serde_json::json!({ "id": "contract" })),
        (
            "surface_swap_placement",
            serde_json::json!({ "session": "contract", "placementA": "a", "placementB": "b" }),
        ),
        ("surface_reconcile", serde_json::json!({})),
        (
            "session_list",
            serde_json::json!({ "projectId": null, "limit": null, "offset": null }),
        ),
        (
            "session_create",
            serde_json::json!({ "projectId": null, "title": "x", "titleSource": "agentTitle", "templateId": null }),
        ),
        (
            "session_rename",
            serde_json::json!({ "id": "contract", "title": "x" }),
        ),
        ("session_archive", serde_json::json!({ "id": "contract" })),
        ("session_delete", serde_json::json!({ "id": "contract" })),
        (
            "session_reorder",
            serde_json::json!({ "id": "contract", "sortOrder": 0 }),
        ),
        (
            "session_layout_set",
            serde_json::json!({ "id": "contract", "layoutJson": "{}" }),
        ),
        (
            "session_layout_get",
            serde_json::json!({ "id": "contract" }),
        ),
        ("session_get", serde_json::json!({ "id": "contract" })),
        (
            "session_list_all",
            serde_json::json!({ "limit": null, "offset": null, "after": null }),
        ),
        (
            "session_get_launch_spec",
            serde_json::json!({ "id": "contract" }),
        ),
        ("session_search", serde_json::json!({ "query": "contract" })),
        ("session_launch", serde_json::json!({ "id": "contract" })),
        (
            "session_apply_launch_spec",
            serde_json::json!({ "id": "contract", "specVersion": 1, "specJson": "{\"version\":1,\"items\":[]}" }),
        ),
        (
            "session_move",
            serde_json::json!({ "id": "contract", "targetProjectId": "contract" }),
        ),
        ("session_duplicate", serde_json::json!({ "id": "contract" })),
        ("session_pin", serde_json::json!({ "id": "contract" })),
        ("session_unpin", serde_json::json!({ "id": "contract" })),
        ("session_restore", serde_json::json!({ "id": "contract" })),
        (
            "session_stop_surfaces",
            serde_json::json!({ "id": "contract" }),
        ),
        ("command_list", serde_json::json!({})),
        (
            "command_create",
            serde_json::json!({ "req": { "name": "x", "cli": "/x", "args": [], "env": {} } }),
        ),
        ("command_get", serde_json::json!({ "id": "contract" })),
        ("command_delete", serde_json::json!({ "id": "contract" })),
        (
            "command_rename",
            serde_json::json!({ "id": "contract", "name": "x" }),
        ),
        (
            "command_edit",
            serde_json::json!({ "id": "contract", "cli": "/x", "args": [], "env": {} }),
        ),
        ("command_pin", serde_json::json!({ "id": "contract" })),
        ("command_unpin", serde_json::json!({ "id": "contract" })),
        (
            "command_duplicate",
            serde_json::json!({ "id": "contract", "name": "copy" }),
        ),
        ("command_seed", serde_json::json!({})),
        (
            "notification_list_unread",
            serde_json::json!({ "limit": null, "offset": null, "after": null }),
        ),
        ("notification_count_unread", serde_json::json!({})),
        (
            "notification_mark_read",
            serde_json::json!({ "id": "contract" }),
        ),
        ("notification_mark_all_read", serde_json::json!({})),
        (
            "notification_disregard",
            serde_json::json!({ "id": "contract" }),
        ),
        ("notification_disregard_all", serde_json::json!({})),
        (
            "notification_snooze",
            serde_json::json!({ "id": "contract", "snoozeUntil": null }),
        ),
        ("notification_prune", serde_json::json!({ "keep": 100 })),
        (
            "setting_get",
            serde_json::json!({ "scope": "global", "projectId": null, "key": "contract" }),
        ),
        (
            "setting_set",
            serde_json::json!({ "scope": "global", "projectId": null, "key": "contract", "valueJson": "1" }),
        ),
        (
            "setting_list",
            serde_json::json!({ "scope": "global", "projectId": null }),
        ),
        (
            "setting_reset",
            serde_json::json!({ "scope": "global", "projectId": null, "key": "contract" }),
        ),
        (
            "setting_resolve",
            serde_json::json!({ "projectId": "contract", "key": "contract" }),
        ),
        (
            "settings_resolve",
            serde_json::json!({ "projectId": "contract" }),
        ),
        ("profile_get_active", serde_json::json!({})),
        ("profile_list", serde_json::json!({})),
        (
            "profile_create",
            serde_json::json!({ "id": "contract", "name": "Contract" }),
        ),
        ("profile_activate", serde_json::json!({ "id": "contract" })),
        (
            "profile_rename",
            serde_json::json!({ "id": "contract", "newName": "Renamed" }),
        ),
        (
            "profile_duplicate",
            serde_json::json!({ "sourceId": "contract", "newId": "contract-copy", "newName": "Copy" }),
        ),
        ("profile_discard", serde_json::json!({ "id": "contract" })),
        ("profile_export", serde_json::json!({ "id": "contract" })),
        (
            "profile_import",
            serde_json::json!({ "profileJson": "{\"id\":\"x\",\"name\":\"X\",\"settings\":{}}" }),
        ),
        ("theme_get_active", serde_json::json!({})),
        ("theme_list", serde_json::json!({})),
        ("theme_activate", serde_json::json!({ "id": "contract" })),
        ("theme_discard", serde_json::json!({ "id": "contract" })),
        ("theme_export", serde_json::json!({ "id": "contract" })),
        (
            "theme_import",
            serde_json::json!({ "id": "contract", "name": "Contract", "origin": "custom", "dataJson": null }),
        ),
        (
            "keybinding_list",
            serde_json::json!({ "defaultsJson": "{}" }),
        ),
        (
            "keybinding_rebind",
            serde_json::json!({ "action": "rename", "chord": "ctrl+r", "defaultsJson": "{}" }),
        ),
        (
            "keybinding_reset",
            serde_json::json!({ "action": "rename", "defaultsJson": "{}" }),
        ),
        (
            "keybinding_reset_all",
            serde_json::json!({ "defaultsJson": "{}" }),
        ),
        (
            "keybinding_resolve",
            serde_json::json!({ "action": "rename", "defaultsJson": "{}" }),
        ),
        ("config_reload", serde_json::json!({})),
        ("notifications_list", serde_json::json!({})),
        (
            "command_center_set_leader",
            serde_json::json!({ "accelerator": "CmdOrCtrl+K" }),
        ),
        (
            "launch_template_create",
            serde_json::json!({ "projectId": "contract", "specVersion": 1, "specJson": "{}" }),
        ),
        (
            "launch_template_list",
            serde_json::json!({ "projectId": "contract" }),
        ),
        (
            "launch_template_get",
            serde_json::json!({ "id": "contract" }),
        ),
        (
            "launch_template_discard",
            serde_json::json!({ "id": "contract" }),
        ),
        (
            "launch_template_apply_spec",
            serde_json::json!({ "id": "contract", "specVersion": 1, "specJson": "{}" }),
        ),
        ("template_list", serde_json::json!({})),
        ("template_get", serde_json::json!({ "id": "contract" })),
        (
            "template_import",
            serde_json::json!({ "name": "contract", "specVersion": 1, "specJson": "{}" }),
        ),
        (
            "template_export",
            serde_json::json!({ "id": "contract", "destPath": "/tmp/contract.json" }),
        ),
        ("template_discard", serde_json::json!({ "id": "contract" })),
        ("template_pin", serde_json::json!({ "id": "contract" })),
        ("template_unpin", serde_json::json!({ "id": "contract" })),
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

/// The `Bus<Ctx>` over a `:memory:` substrate is managed and accessible. This is the
/// wiring contract: the context composes correctly (migrations applied, runtime injected)
/// and the bus dispatches without panicking. Business logic is not exercised here.
#[test]
#[serial]
fn memory_ctx_bus_is_managed_and_wired() {
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var(tillerd_paths::ENV_TILLERD_DIR, tmp.path());
    std::env::set_var(tillerd_paths::ENV_DAEMON_BIN, "/dev/null");

    let app = contract_app();

    // The bus is managed: resolving it does not panic.
    let _bus: tauri::State<'_, Bus<Ctx>> = app.state();

    std::env::remove_var(tillerd_paths::ENV_TILLERD_DIR);
    std::env::remove_var(tillerd_paths::ENV_DAEMON_BIN);
}

/// Generate the TypeScript bindings for the tauri commands. Runs the specta
/// export without launching a full app. The output path is fixed at build time
/// via `CARGO_MANIFEST_DIR`.
#[test]
fn export_tauri_bindings() {
    let out = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/client-bindings/src/tauri_bindings.gen.ts"
    );
    // Run on a thread with an expanded stack to avoid overflow during specta
    // type traversal across the large command surface.
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            crate::specta_builder()
                .export(
                    crate::specta_export::ObjectParamTs(
                        specta_typescript::Typescript::default().header(crate::GEN_HEADER),
                    ),
                    out,
                )
                .expect("tauri bindings export succeeded");
        })
        .expect("spawn export thread")
        .join()
        .expect("export thread joined");

    assert!(
        std::path::Path::new(out).exists(),
        "tauri_bindings.ts was written to {out}"
    );
}
