mod bootstrap;
mod bridge;
mod diag;
mod files;
mod gate_admin;
mod orchestrator;
mod paths;
mod store;
mod supervisor;

use tauri::Manager;

use bridge::BridgeState;
use store::StoreState;
use supervisor::SupervisorState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(BridgeState::default())
        .manage(StoreState::load())
        .manage(SupervisorState::default())
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
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                supervisor::shutdown_owned(&app_handle.state::<SupervisorState>());
            }
        });
}
