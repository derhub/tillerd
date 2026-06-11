use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use orchestrator::persistence::{SqliteStore, Store};
use orchestrator::supervision::{ProcessSupervisor, ServiceSpec, SpawnFn};
use orchestrator::{boot, EventSink, Orchestrator, Status};
use process_launch::{tillerd_dir, LaunchError};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::paths::{resolve_daemon_bin, resolve_gate_bin};

pub const ORCHESTRATOR_STATUS_EVENT: &str = "orchestrator://status";

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum StatusWire {
    Booting,
    OpeningStore,
    Supervising,
    Ready,
    Failed { reason: String },
}

impl From<&Status> for StatusWire {
    fn from(status: &Status) -> Self {
        match status {
            Status::Booting => StatusWire::Booting,
            Status::OpeningStore => StatusWire::OpeningStore,
            Status::Supervising => StatusWire::Supervising,
            Status::Ready => StatusWire::Ready,
            Status::Failed { reason } => StatusWire::Failed {
                reason: reason.clone(),
            },
        }
    }
}

pub struct OrchestratorState {
    status: Arc<Mutex<StatusWire>>,
    orchestrator: Arc<Mutex<Option<Orchestrator>>>,
}

impl Default for OrchestratorState {
    fn default() -> Self {
        Self {
            status: Arc::new(Mutex::new(StatusWire::Booting)),
            orchestrator: Arc::new(Mutex::new(None)),
        }
    }
}

struct TauriEventSink {
    app: AppHandle,
    status: Arc<Mutex<StatusWire>>,
}

impl EventSink for TauriEventSink {
    fn emit(&self, event: &Status) {
        let wire = StatusWire::from(event);
        *self.status.lock().unwrap() = wire.clone();
        let _ = self.app.emit(ORCHESTRATOR_STATUS_EVENT, wire);
    }
}

#[tauri::command]
pub fn orchestrator_status(state: State<'_, OrchestratorState>) -> StatusWire {
    state.status.lock().unwrap().clone()
}

fn spawn_fn(resolve: fn() -> Option<PathBuf>, name: &'static str, dir: PathBuf) -> SpawnFn {
    Box::new(move || {
        let bin = resolve().ok_or_else(|| LaunchError::BinaryNotFound(name.to_string()))?;
        Command::new(&bin)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .env("TILLERD_DIR", &dir)
            .spawn()
            .map(|child| child.id())
            .map_err(|e| LaunchError::SpawnFailed(e.to_string()))
    })
}

fn build_supervisor() -> ProcessSupervisor {
    let dir = tillerd_dir();
    let version = env!("CARGO_PKG_VERSION").to_string();
    ProcessSupervisor::new()
        .service(
            ServiceSpec {
                name: "gate".to_string(),
                manifest_path: dir.join("gate.json"),
                socket_path: dir.join("gate.sock"),
                version: version.clone(),
            },
            spawn_fn(resolve_gate_bin, "tillerd-gate", dir.clone()),
        )
        .service(
            ServiceSpec {
                name: "daemon".to_string(),
                manifest_path: dir.join("daemon.json"),
                socket_path: dir.join("daemon.sock"),
                version,
            },
            spawn_fn(resolve_daemon_bin, "tillerd-daemon", dir),
        )
}

pub fn spawn_boot(app: AppHandle, state: &OrchestratorState) {
    let status = state.status.clone();
    let slot = state.orchestrator.clone();
    std::thread::spawn(move || {
        // Keep a clone for post-boot work; the original moves into the sink.
        let app_for_surface = app.clone();
        let sink = TauriEventSink { app, status };
        let mut supervisor = build_supervisor();
        let open_store = || SqliteStore::open_default().map(|s| Box::new(s) as Box<dyn Store>);
        match boot(open_store, &mut supervisor, &sink) {
            Ok(orchestrator) => {
                // Wire up the terminal-surface layer before storing the orchestrator
                // so the SurfaceState is available before any IPC command fires.
                crate::surface_host::register(&app_for_surface, orchestrator.store_arc());
                *slot.lock().unwrap() = Some(orchestrator);
            }
            Err(error) => {
                eprintln!("orchestrator boot failed: {error}");
            }
        }
    });
}
