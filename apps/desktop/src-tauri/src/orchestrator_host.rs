use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use orchestrator::persistence::{SqliteStore, Store};
use orchestrator::supervision::{ProcessSupervisor, ServiceSpec, SpawnFn, SpawnTiming};
use orchestrator::{
    boot, read_service_health, EventSink, HealthSpec, Orchestrator, ServiceHealth, ServiceState,
    Status,
};
use process_launch::LaunchError;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::notification_host;
use tillerd_paths::{
    daemon_socket_in, gate_socket_in, manifest_in, resolve_daemon_bin, resolve_gate_bin,
    runtime_dir,
};

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

/// A service's state on the wire. Additive read-only health surface; mirrors
/// `orchestrator::ServiceState`.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServiceStateWire {
    Starting,
    Ready,
    Draining,
    VersionMismatch,
    Unavailable,
}

impl From<ServiceState> for ServiceStateWire {
    fn from(state: ServiceState) -> Self {
        match state {
            ServiceState::Starting => ServiceStateWire::Starting,
            ServiceState::Ready => ServiceStateWire::Ready,
            ServiceState::Draining => ServiceStateWire::Draining,
            ServiceState::VersionMismatch => ServiceStateWire::VersionMismatch,
            ServiceState::Unavailable => ServiceStateWire::Unavailable,
        }
    }
}

/// One service's health on the wire.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceHealthWire {
    pub name: String,
    pub version: Option<String>,
    pub state: ServiceStateWire,
}

impl From<ServiceHealth> for ServiceHealthWire {
    fn from(health: ServiceHealth) -> Self {
        Self {
            name: health.name,
            version: health.version,
            state: health.state.into(),
        }
    }
}

pub struct OrchestratorState {
    status: Arc<Mutex<StatusWire>>,
    orchestrator: Arc<Mutex<Option<Orchestrator>>>,
}

impl OrchestratorState {
    /// Return the store if the orchestrator has booted, or `None`.
    pub fn store_arc(&self) -> Option<Arc<dyn Store>> {
        self.orchestrator
            .lock()
            .unwrap()
            .as_ref()
            .map(|o| o.store_arc())
    }
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
    /// Filled once the orchestrator has booted; until then status notifications are not
    /// persisted (the store does not exist yet).
    store: Arc<Mutex<Option<Arc<dyn Store>>>>,
    /// Previous health snapshot for diffing; seeded from the first post-boot read.
    prev_health: Arc<Mutex<Option<Vec<ServiceHealthWire>>>>,
}

impl TauriEventSink {
    /// Persist service-up/down (health diff) and ready/failed notifications for a status change.
    /// No-op until the store is available; the in-boot `Ready` is recorded by `spawn_boot` instead.
    fn record_status_notifications(&self, event: &Status) {
        let store = self.store.lock().unwrap().clone();
        let Some(store) = store else {
            return;
        };
        let ts = notification_host::now_ms();
        let current = service_health_snapshot();
        let prev = self.prev_health.lock().unwrap().clone();
        if let Some(prev) = prev {
            for n in notification_host::health_change_notifications(&prev, &current, ts) {
                notification_host::record(&self.app, store.as_ref(), n);
            }
        }
        *self.prev_health.lock().unwrap() = Some(current);
        match event {
            Status::Ready => notification_host::record(
                &self.app,
                store.as_ref(),
                notification_host::orchestrator_status(true, None, ts),
            ),
            Status::Failed { reason } => notification_host::record(
                &self.app,
                store.as_ref(),
                notification_host::orchestrator_status(false, Some(reason), ts),
            ),
            _ => {}
        }
    }
}

impl EventSink for TauriEventSink {
    fn emit(&self, event: &Status) {
        let wire = StatusWire::from(event);
        *self.status.lock().unwrap() = wire.clone();
        let _ = self.app.emit(ORCHESTRATOR_STATUS_EVENT, wire);
        self.record_status_notifications(event);
    }
}

#[tauri::command]
pub fn orchestrator_status(state: State<'_, OrchestratorState>) -> StatusWire {
    state.status.lock().unwrap().clone()
}

/// The manifest path each supervised service writes, kept here so the health read
/// and `build_supervisor` resolve the same locations. Read-only health derives
/// from these manifests (ADR-0028 discovery source); no socket or route is opened.
fn service_health_specs() -> Vec<HealthSpec> {
    let dir = runtime_dir();
    let version = env!("CARGO_PKG_VERSION").to_string();
    // Names match each service's `service.name` in the structured logs
    // (`tillerd-gate` / `tillerd-daemon`) so a row's logs link filters correctly.
    vec![
        HealthSpec {
            name: "tillerd-gate".to_string(),
            manifest_path: dir.join("gate.json"),
            expected_version: version.clone(),
        },
        HealthSpec {
            name: "tillerd-daemon".to_string(),
            manifest_path: manifest_in(&dir),
            expected_version: version,
        },
    ]
}

/// Read-only per-service health (gate, daemon), read live from each manifest so a
/// service that is down, mismatched, or draining is observable even when the
/// orchestrator never reached `ready`. The renderer re-queries this on each
/// `orchestrator://status` event; there is no separate health event.
fn service_health_snapshot() -> Vec<ServiceHealthWire> {
    read_service_health(&service_health_specs(), process_launch::pid_is_alive)
        .into_iter()
        .map(ServiceHealthWire::from)
        .collect()
}

#[tauri::command]
pub fn service_health() -> Vec<ServiceHealthWire> {
    service_health_snapshot()
}

fn spawn_fn(resolve: fn() -> Option<PathBuf>, name: &'static str, dir: PathBuf) -> SpawnFn {
    Box::new(move || {
        let bin = resolve().ok_or_else(|| LaunchError::BinaryNotFound(name.to_string()))?;
        Command::new(&bin)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .env(tillerd_paths::ENV_TILLERD_DIR, &dir)
            .spawn()
            .map(|child| child.id())
            .map_err(|e| LaunchError::SpawnFailed(e.to_string()))
    })
}

fn build_supervisor() -> ProcessSupervisor {
    let dir = runtime_dir();
    let version = env!("CARGO_PKG_VERSION").to_string();
    // Cold-starting a freshly-built service can exceed the 10s default under load; fail-fast on a
    // dead child keeps a genuine crash from waiting this out.
    ProcessSupervisor::new()
        .with_timing(SpawnTiming {
            startup_timeout: Duration::from_secs(30),
            poll_interval: Duration::from_millis(100),
        })
        .service(
            ServiceSpec {
                name: "gate".to_string(),
                manifest_path: dir.join("gate.json"),
                socket_path: gate_socket_in(&dir),
                version: version.clone(),
            },
            spawn_fn(resolve_gate_bin, "tillerd-gate", dir.clone()),
        )
        .service(
            ServiceSpec {
                name: "daemon".to_string(),
                manifest_path: manifest_in(&dir),
                socket_path: daemon_socket_in(&dir),
                version,
            },
            spawn_fn(resolve_daemon_bin, "tillerd-daemon", dir),
        )
}

pub fn spawn_boot(app: AppHandle, state: &OrchestratorState) {
    let status = state.status.clone();
    let slot = state.orchestrator.clone();
    std::thread::spawn(move || {
        let app_for_surface = app.clone();
        let sink = TauriEventSink {
            app,
            status,
            store: Arc::new(Mutex::new(None)),
            prev_health: Arc::new(Mutex::new(None)),
        };
        let mut supervisor = build_supervisor();
        let open_store = || SqliteStore::open_default().map(|s| Box::new(s) as Box<dyn Store>);
        match boot(open_store, &mut supervisor, &sink) {
            Ok(orchestrator) => {
                let store = orchestrator.store_arc();
                // The in-boot `Ready` emit ran before the store was shared with the sink; wire it
                // up now, seed the health baseline, and record the ready notification.
                *sink.store.lock().unwrap() = Some(store.clone());
                *sink.prev_health.lock().unwrap() = Some(service_health_snapshot());
                notification_host::record(
                    &app_for_surface,
                    store.as_ref(),
                    notification_host::orchestrator_status(true, None, notification_host::now_ms()),
                );
                // Register the surface layer before stashing the orchestrator so
                // SurfaceState exists before any IPC command can fire.
                crate::surface_host::register(&app_for_surface, store);
                *slot.lock().unwrap() = Some(orchestrator);
            }
            Err(error) => {
                notification_host::emit_only(
                    &app_for_surface,
                    notification_host::orchestrator_status(
                        false,
                        Some(&error.to_string()),
                        notification_host::now_ms(),
                    ),
                );
                eprintln!("orchestrator boot failed: {error}");
            }
        }
    });
}
