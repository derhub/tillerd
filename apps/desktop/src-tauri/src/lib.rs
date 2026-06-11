mod bootstrap;
mod bridge;
mod daemon_session;
mod diag;
mod files;
mod gate_admin;
mod orchestrator_host;
mod paths;
mod store;
mod supervisor;
mod surface_host;

use tauri::Manager;

use bridge::BridgeState;
use orchestrator_host::OrchestratorState;
use store::StoreState;
use supervisor::SupervisorState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            bootstrap::agent_bootstrap,
            orchestrator_host::orchestrator_status,
            surface_host::surface_create,
            surface_host::surface_input,
            surface_host::surface_resize,
            surface_host::surface_detach,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                supervisor::shutdown_owned(&app_handle.state::<SupervisorState>());
            }
        });
}
