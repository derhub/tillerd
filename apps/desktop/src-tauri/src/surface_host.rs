//! Terminal-surface Tauri host: bridges the orchestrator's [`SurfaceApi`] to the
//! renderer via IPC channels and typed app events.
//!
//! # Design
//!
//! The byte stream for each surface travels over a [`tauri::ipc::Channel<Vec<u8>>`]
//! (the same pattern as the daemon byte bridge in `bridge.rs`).  Status and exit
//! notifications are emitted as named app events so the renderer can handle them
//! without polling.
//!
//! A [`SurfaceState`] is registered as Tauri managed state after the orchestrator
//! boots successfully.  It holds an [`Arc<SurfaceApi>`] and a shared channel map.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use orchestrator::persistence::SurfaceId;
use orchestrator::surface::transport::default_daemon_socket;
use orchestrator::surface::{SurfaceApi, SurfaceEventSink};
use tauri::{AppHandle, Emitter, Manager, State};

// ── channel registry ─────────────────────────────────────────────────────────

/// Map from surface-id string to the IPC channel delivering raw PTY bytes.
pub type SurfaceChannels = Arc<Mutex<HashMap<String, tauri::ipc::Channel<Vec<u8>>>>>;

// ── event sink ───────────────────────────────────────────────────────────────

/// Delivers surface events from the async runtime back to the renderer.
///
/// Byte output is forwarded over the surface's [`tauri::ipc::Channel`]; status
/// and exit transitions are emitted as named app events.
pub struct TauriSurfaceSink {
    channels: SurfaceChannels,
    app: AppHandle,
}

impl SurfaceEventSink for TauriSurfaceSink {
    /// Forward raw PTY bytes to the surface's IPC channel.
    fn on_bytes(&self, surface: &SurfaceId, bytes: &[u8]) {
        let channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ch) = channels.get(surface.as_str()) {
            let _ = ch.send(bytes.to_vec());
        }
    }

    /// Emit a `surface://status` event to the renderer.
    fn on_status(&self, surface: &SurfaceId, status: &str) {
        let _ = self.app.emit(
            "surface://status",
            serde_json::json!({
                "surfaceId": surface.as_str(),
                "status": status,
            }),
        );
    }

    /// Emit a `surface://exit` event and remove the surface's channel.
    fn on_exit(&self, surface: &SurfaceId, qualifier: &str) {
        let _ = self.app.emit(
            "surface://exit",
            serde_json::json!({
                "surfaceId": surface.as_str(),
                "qualifier": qualifier,
            }),
        );
        let mut channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());
        channels.remove(surface.as_str());
    }
}

// ── managed state ────────────────────────────────────────────────────────────

/// Tauri managed state for terminal surfaces.
pub struct SurfaceState {
    /// The orchestrator's surface API, shared across commands.
    pub api: Arc<SurfaceApi>,
    /// Channel registry, also shared with the sink so `on_exit` can clean up.
    pub channels: SurfaceChannels,
}

/// Build and register [`SurfaceState`] from a live orchestrator store.
///
/// Called from [`crate::orchestrator_host::spawn_boot`] after the orchestrator
/// boots successfully.  Runs inside a `std::thread`, so async work is handed
/// off to `tauri::async_runtime::spawn`.
pub fn register(app: &AppHandle, store: std::sync::Arc<dyn orchestrator::persistence::Store>) {
    let channels: SurfaceChannels = Arc::new(Mutex::new(HashMap::new()));
    let sink = Arc::new(TauriSurfaceSink {
        channels: channels.clone(),
        app: app.clone(),
    });
    let api = Arc::new(SurfaceApi::new(store, sink, default_daemon_socket()));

    // Kick off resume_all in the async runtime; failures are non-fatal.
    let api_for_resume = api.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = api_for_resume.resume_all().await {
            eprintln!("surface resume_all failed (non-fatal): {e}");
        }
    });

    app.manage(SurfaceState { api, channels });
}

// ── IPC commands ─────────────────────────────────────────────────────────────

/// Create a terminal surface and return its id.
///
/// The `channel` receives raw PTY bytes for this surface.  The channel is
/// registered *before* the daemon session is created so no initial bytes are
/// lost.
#[tauri::command]
pub async fn surface_create(
    state: State<'_, SurfaceState>,
    channel: tauri::ipc::Channel<Vec<u8>>,
    cols: u16,
    rows: u16,
    cwd: Option<String>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    // Register the channel before calling create so no initial output is lost.
    {
        let mut channels = state
            .channels
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        channels.insert(id.clone(), channel);
    }
    state
        .api
        .create_terminal_surface(SurfaceId::from_string(id.clone()), cols, rows, cwd)
        .await
        .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Forward input bytes to a surface's PTY.
#[tauri::command]
pub async fn surface_input(
    state: State<'_, SurfaceState>,
    surface_id: String,
    bytes: Vec<u8>,
) -> Result<(), String> {
    state
        .api
        .input(&SurfaceId::from_string(surface_id), &bytes)
        .await
        .map_err(|e| e.to_string())
}

/// Resize a surface's terminal.
#[tauri::command]
pub async fn surface_resize(
    state: State<'_, SurfaceState>,
    surface_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    state
        .api
        .resize(&SurfaceId::from_string(surface_id), cols, rows)
        .await
        .map_err(|e| e.to_string())
}

/// Detach a surface, leaving its daemon session alive for later resume.
///
/// Removes the IPC channel (bytes will no longer be routed) then asks the
/// runtime to detach.
#[tauri::command]
pub async fn surface_detach(
    state: State<'_, SurfaceState>,
    surface_id: String,
) -> Result<(), String> {
    {
        let mut channels = state
            .channels
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        channels.remove(&surface_id);
    }
    state
        .api
        .detach(&SurfaceId::from_string(surface_id))
        .await
        .map_err(|e| e.to_string())
}
