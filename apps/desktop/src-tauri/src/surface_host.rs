use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use orchestrator::persistence::{SessionId, SurfaceId};
use orchestrator::surface::transport::default_daemon_socket;
use orchestrator::surface::{SurfaceApi, SurfaceEventSink};
use tauri::{AppHandle, Emitter, Manager, State};

pub type SurfaceChannels = Arc<Mutex<HashMap<String, tauri::ipc::Channel<Vec<u8>>>>>;

pub struct TauriSurfaceSink {
    channels: SurfaceChannels,
    app: AppHandle,
}

impl SurfaceEventSink for TauriSurfaceSink {
    fn on_bytes(&self, surface: &SurfaceId, bytes: &[u8]) {
        let channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ch) = channels.get(surface.as_str()) {
            let _ = ch.send(bytes.to_vec());
        }
    }

    fn on_status(&self, surface: &SurfaceId, status: &str) {
        let _ = self.app.emit(
            "surface://status",
            serde_json::json!({ "surfaceId": surface.as_str(), "status": status }),
        );
    }

    fn on_exit(&self, surface: &SurfaceId, qualifier: &str) {
        let _ = self.app.emit(
            "surface://exit",
            serde_json::json!({ "surfaceId": surface.as_str(), "qualifier": qualifier }),
        );
        let mut channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());
        channels.remove(surface.as_str());
    }

    fn on_error(&self, surface: &SurfaceId, reason: &str) {
        let _ = self.app.emit(
            "surface:error",
            serde_json::json!({ "surfaceId": surface.as_str(), "reason": reason }),
        );
    }
}

pub struct SurfaceState {
    pub api: Arc<SurfaceApi>,
    pub channels: SurfaceChannels,
}

pub fn register(app: &AppHandle, store: Arc<dyn orchestrator::persistence::Store>) {
    let channels: SurfaceChannels = Arc::new(Mutex::new(HashMap::new()));
    let sink = Arc::new(TauriSurfaceSink {
        channels: channels.clone(),
        app: app.clone(),
    });
    let api = Arc::new(SurfaceApi::new(store, sink, default_daemon_socket()));

    let api_for_resume = api.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = api_for_resume.resume_all().await {
            eprintln!("surface resume_all failed (non-fatal): {e}");
        }
    });

    app.manage(SurfaceState { api, channels });
}

#[tauri::command]
pub async fn surface_create(
    state: State<'_, SurfaceState>,
    channel: tauri::ipc::Channel<Vec<u8>>,
    session_id: String,
    placement: String,
    cols: u16,
    rows: u16,
    cwd: Option<String>,
) -> Result<String, String> {
    let session = SessionId::from_string(session_id);

    // Revisit: re-attach to the session's existing surface at this placement (resume replays its
    // scrollback) rather than spawn a fresh one; a stale surface (shell exited) falls through.
    if let Some(existing) = state
        .api
        .find_session_surface_by_placement(&session, &placement)
        .map_err(|e| e.to_string())?
    {
        // Register the channel before resume so no replayed output is lost.
        state
            .channels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(existing.as_str().to_string(), channel.clone());
        // Drop any lingering proxy first so resume does a fresh subscribe (replays scrollback)
        // instead of its idempotent no-op.
        let _ = state.api.detach(&existing).await;
        match state.api.resume_surface(&existing).await {
            Ok(()) => return Ok(existing.as_str().to_string()),
            Err(_) => {
                state
                    .channels
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(existing.as_str());
                let _ = state.api.remove(&existing).await;
            }
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    // Register the channel before create so no initial output is lost.
    state
        .channels
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(id.clone(), channel);
    state
        .api
        .create_terminal_surface(
            session,
            SurfaceId::from_string(id.clone()),
            placement,
            cols,
            rows,
            cwd,
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Spawn a surface into a session: diverge the session's launch spec and return the minted
/// placement. The renderer then mounts a pane at that placement, which calls `surface_create`.
#[tauri::command]
pub async fn surface_spawn(
    state: State<'_, SurfaceState>,
    session_id: String,
) -> Result<String, String> {
    state
        .api
        .spawn_surface(&SessionId::from_string(session_id))
        .map_err(|e| e.to_string())
}

/// Close a surface: drop its launch item from the session spec and hard-remove it (terminate PTY).
#[tauri::command]
pub async fn surface_close(
    state: State<'_, SurfaceState>,
    session_id: String,
    surface_id: String,
) -> Result<(), String> {
    let surface = SurfaceId::from_string(surface_id);
    state
        .channels
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(surface.as_str());
    state
        .api
        .remove_surface(&SessionId::from_string(session_id), &surface)
        .await
        .map_err(|e| e.to_string())
}

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

#[tauri::command]
pub async fn surface_detach(
    state: State<'_, SurfaceState>,
    surface_id: String,
) -> Result<(), String> {
    state
        .channels
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&surface_id);
    state
        .api
        .detach(&SurfaceId::from_string(surface_id))
        .await
        .map_err(|e| e.to_string())
}
