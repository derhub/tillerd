use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use orchestrator::persistence::{SessionId, Store, SurfaceId};
use orchestrator::surface::transport::default_daemon_socket;
use orchestrator::surface::{SurfaceApi, SurfaceEventSink};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::notification_host;

pub type SurfaceChannels = Arc<Mutex<HashMap<String, tauri::ipc::Channel<Vec<u8>>>>>;

pub struct TauriSurfaceSink {
    channels: SurfaceChannels,
    app: AppHandle,
    store: Arc<dyn Store>,
}

impl TauriSurfaceSink {
    /// The session a surface belongs to, for click-through context. `None` if the row is gone.
    fn session_of(&self, surface: &SurfaceId) -> Option<String> {
        self.store
            .get_surface(surface)
            .ok()
            .flatten()
            .map(|s| s.session_id.as_str().to_string())
    }
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
        notification_host::record(
            &self.app,
            self.store.as_ref(),
            notification_host::surface_stopped(
                surface.as_str(),
                self.session_of(surface),
                qualifier,
                notification_host::now_ms(),
            ),
        );
        let mut channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());
        channels.remove(surface.as_str());
    }

    fn on_error(&self, surface: &SurfaceId, reason: &str) {
        let _ = self.app.emit(
            "surface:error",
            serde_json::json!({ "surfaceId": surface.as_str(), "reason": reason }),
        );
        notification_host::record(
            &self.app,
            self.store.as_ref(),
            notification_host::surface_error(
                surface.as_str(),
                self.session_of(surface),
                reason,
                notification_host::now_ms(),
            ),
        );
    }
}

pub struct SurfaceState {
    pub api: Arc<SurfaceApi>,
    pub channels: SurfaceChannels,
    pub store: Arc<dyn Store>,
}

pub fn register(app: &AppHandle, store: Arc<dyn orchestrator::persistence::Store>) {
    let channels: SurfaceChannels = Arc::new(Mutex::new(HashMap::new()));
    let sink = Arc::new(TauriSurfaceSink {
        channels: channels.clone(),
        app: app.clone(),
        store: store.clone(),
    });
    let api = Arc::new(SurfaceApi::new(
        store.clone(),
        sink,
        default_daemon_socket(),
    ));

    let api_for_resume = api.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = api_for_resume.resume_all().await {
            eprintln!("surface resume_all failed (non-fatal): {e}");
        }
    });

    app.manage(SurfaceState {
        api,
        channels,
        store,
    });
}

// The arg list mirrors the renderer's IPC call shape (channel + placement + geometry); the
// injected `AppHandle` tips it over clippy's threshold.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn surface_create<R: tauri::Runtime>(
    app: AppHandle<R>,
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
    let session_str = session.as_str().to_string();
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
    notification_host::record(
        &app,
        state.store.as_ref(),
        notification_host::surface_started(&id, &session_str, notification_host::now_ms()),
    );
    Ok(id)
}

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

// Keyed by placement, not surface id -- a panel binds a surface by placement.
#[tauri::command]
pub async fn surface_close(
    state: State<'_, SurfaceState>,
    session_id: String,
    placement: String,
) -> Result<(), String> {
    let session = SessionId::from_string(session_id);
    let Some(surface) = state
        .api
        .find_session_surface_by_placement(&session, &placement)
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    state
        .channels
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(surface.as_str());
    state
        .api
        .remove_surface(&session, &surface)
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
