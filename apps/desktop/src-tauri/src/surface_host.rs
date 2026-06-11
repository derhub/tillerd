use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use contracts::ContentEvent;
use orchestrator::persistence::{SessionId, SurfaceId};
use orchestrator::surface::transport::default_daemon_socket;
use orchestrator::surface::{SurfaceApi, SurfaceEventSink};
use tauri::{AppHandle, Emitter, Manager, State};
use tillerd_paths::{gate_socket, resolve_notify_bin};

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

    fn on_content(&self, surface: &SurfaceId, event: &ContentEvent) {
        let _ = self.app.emit(
            "surface:content",
            serde_json::json!({ "surfaceId": surface.as_str(), "event": event }),
        );
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
    let api = Arc::new(SurfaceApi::with_gate_socket(
        store,
        sink,
        default_daemon_socket(),
        gate_socket(),
    ));

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
    cols: u16,
    rows: u16,
    cwd: Option<String>,
) -> Result<String, String> {
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
            SessionId::from_string(session_id),
            SurfaceId::from_string(id.clone()),
            cols,
            rows,
            cwd,
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(id)
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

#[tauri::command]
pub async fn surface_create_agent(
    state: State<'_, SurfaceState>,
    channel: tauri::ipc::Channel<Vec<u8>>,
    session_id: String,
    cols: u16,
    rows: u16,
    cwd: Option<String>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    state
        .channels
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(id.clone(), channel);

    let agent_home = default_agent_home();
    let notify_cmd = resolve_notify_bin()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "tillerd-notify".to_string());

    state
        .api
        .create_agent_surface(
            SessionId::from_string(session_id),
            SurfaceId::from_string(id.clone()),
            &agent_home,
            &notify_cmd,
            cols,
            rows,
            cwd,
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(id)
}

fn default_agent_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".claude")
}
