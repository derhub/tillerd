mod bridge;
#[cfg(test)]
mod command_contract;
mod daemon_session;
mod diag;
mod files;
mod gate_admin;
mod orchestrator_host;
mod store;
mod supervisor;
mod surface_host;
mod workspace_host;

use tauri::Manager;

use bridge::BridgeState;
use orchestrator_host::OrchestratorState;
use store::StoreState;
use supervisor::SupervisorState;

/// The app's real `Context` (config, embedded frontend, resolved ACL). `generate_context!` may
/// expand only once per binary, so this single call is shared by `run()` and the command-contract
/// test — the test needs the real ACL (the bundled `default.json`) to drive commands through the
/// live IPC path, which a `mock_context` (empty ACL) cannot do.
fn app_context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(feature = "webdriver")]
    let builder = builder.plugin(tauri_plugin_webdriver::init());
    builder
        .manage(BridgeState::default())
        .manage(StoreState::load())
        .manage(SupervisorState::default())
        .manage(OrchestratorState::default())
        .setup(|app| {
            // Construct and boot the single embedded orchestrator instance; it
            // streams lifecycle events to the renderer and reaches `ready`.
            let handle = app.handle().clone();
            let state = app.state::<OrchestratorState>();
            orchestrator_host::spawn_boot(handle, state.inner());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bridge::daemon_connect,
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
            orchestrator_host::orchestrator_status,
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
        .build(app_context())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                supervisor::shutdown_owned(&app_handle.state::<SupervisorState>());
            }
        });
}
