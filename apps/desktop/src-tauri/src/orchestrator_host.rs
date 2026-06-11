//! Host embedding of the runtime-agnostic orchestrator crate (ADR-0022).
//!
//! Constructs one orchestrator instance at startup, supervises the gate and
//! daemon, opens the product store, and binds the orchestrator's `status()`
//! request method and its lifecycle event stream to a Tauri command and the host
//! event channel. The renderer observes `ready` through these via the SDK client.

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

/// The event name the orchestrator lifecycle stream is emitted under.
pub const ORCHESTRATOR_STATUS_EVENT: &str = "orchestrator://status";

/// The lifecycle status as a wire value for the renderer. Mirrors
/// [`orchestrator::Status`]; the SDK hand-authors the matching TypeScript type
/// (wire-type generation is deferred to 0.1.4).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum StatusWire {
    /// Boot has started.
    Booting,
    /// Opening and migrating the store.
    OpeningStore,
    /// Supervising the shared services.
    Supervising,
    /// Store open and every service available.
    Ready,
    /// Boot failed; carries the typed reason. Terminal.
    Failed {
        /// Why boot failed.
        reason: String,
    },
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

/// Host state: the latest lifecycle status (observable before boot completes) and
/// the single booted orchestrator instance.
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

/// The host [`EventSink`]: forwards each lifecycle event to the renderer over the
/// Tauri event channel and records the latest status for the status query.
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

/// The orchestrator `status()` request method, bound as a host command.
#[tauri::command]
pub fn orchestrator_status(state: State<'_, OrchestratorState>) -> StatusWire {
    state.status.lock().unwrap().clone()
}

/// Build a spawn closure for a service: resolve its binary lazily (a missing
/// binary becomes a typed `BinaryNotFound`, so boot fails honestly rather than
/// hanging) and launch it detached with the runtime dir in its environment.
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

/// Build the supervisor for the shared services. Both the gate and the daemon are
/// required; the wanted version is this build's version (every workspace binary
/// shares it pre-v1), so a running instance is adopted only when it is the same
/// build.
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

/// Construct and boot the single orchestrator instance on a background thread,
/// streaming lifecycle events to the renderer and storing the booted instance so
/// later request methods can reach the backend.
pub fn spawn_boot(app: AppHandle, state: &OrchestratorState) {
    let status = state.status.clone();
    let slot = state.orchestrator.clone();
    std::thread::spawn(move || {
        let sink = TauriEventSink { app, status };
        let mut supervisor = build_supervisor();
        let open_store = || SqliteStore::open_default().map(|s| Box::new(s) as Box<dyn Store>);
        match boot(open_store, &mut supervisor, &sink) {
            Ok(orchestrator) => {
                *slot.lock().unwrap() = Some(orchestrator);
            }
            Err(error) => {
                // boot already emitted the terminal Failed event; record the cause.
                eprintln!("orchestrator boot failed: {error}");
            }
        }
    });
}
