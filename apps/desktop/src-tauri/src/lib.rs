mod bridge;
#[cfg(test)]
mod command_contract;
mod daemon_session;
mod diag;
mod files;
mod gate_admin;
mod menu;
mod notification_host;
mod orchestrator_host;
mod settings_host;
mod store;
mod supervisor;
mod surface_host;
mod transport;
mod window_host;

use tauri::Manager;

use transport::macros::collect_transport;

use bridge::BridgeState;
use orchestrator_host::OrchestratorState;
use store::StoreState;
use supervisor::SupervisorState;

/// The app's real `Context` (config, embedded frontend, resolved ACL). `generate_context!` may
/// expand only once per binary, so this single call is shared by `run()` and the command-contract
/// test -- the test needs the real ACL (the bundled `default.json`) to drive commands through the
/// live IPC path, which a `mock_context` (empty ACL) cannot do.
fn app_context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let dir = tillerd_paths::runtime_dir();
    let (_guard, root) = tillerd_paths::logging::init_file_tracing(
        "tillerd-desktop",
        env!("CARGO_PKG_VERSION"),
        &dir,
    );
    let _root = root.entered();

    let builder = tauri::Builder::default();
    #[cfg(feature = "webdriver")]
    let builder = builder.plugin(tauri_plugin_webdriver::init());
    // Save/restore window size, position, and maximized state across relaunch. The default
    // builder auto-saves on exit and restores on launch; no manual save/restore calls needed.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_window_state::Builder::default().build());
    // Native OS notification banners for background (unfocused) events (roadmap 0.0.10).
    let builder = builder.plugin(tauri_plugin_notification::init());
    builder
        .manage(BridgeState::default())
        .manage(StoreState::load())
        .manage(SupervisorState::default())
        .manage(OrchestratorState::default())
        .manage(menu::LeaderMenuState::default())
        .setup(|app| {
            // Native menu: the platform default (app / Edit / View / Window / Help) with Logs and
            // Command Center entries in the View submenu. Logs routes the renderer to /logs; the
            // Command Center accelerator is the leader key (fires over terminal focus) and emits
            // "command-center:open".
            menu::install_menu(app)?;

            // Construct and boot the single embedded orchestrator instance; it
            // streams lifecycle events to the renderer and reaches `ready`.
            let handle = app.handle().clone();
            let state = app.state::<OrchestratorState>();
            orchestrator_host::spawn_boot(handle, state.inner());
            Ok(())
        })
        .invoke_handler(collect_transport!())
        .build(app_context())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // Shutdown is bound to app exit (ExitRequested fires only when the LAST window closes),
            // never per-window-close -- so closing a parent window while a detached child remains
            // open leaves the owned daemon running (roadmap 0.0.11, desktop-shell spec).
            if let tauri::RunEvent::ExitRequested { .. } = event {
                supervisor::shutdown_owned(&app_handle.state::<SupervisorState>());
            }
        });
}
