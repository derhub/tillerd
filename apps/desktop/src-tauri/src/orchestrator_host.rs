use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use orchestrator::app::command::SeedCommands;
use orchestrator::app::notification::OrchestratorStatus;
use orchestrator::app::surface::ReconcileSurfaces;
use orchestrator::supervision::{
    all_available, ProcessSupervisor, ServiceSpec, SpawnFn, SpawnTiming, Supervise,
};
use orchestrator::{
    build_bus, read_service_health, Config, HealthSpec, ServiceHealth, ServiceState,
};
use process_launch::LaunchError;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::transport::notification;
use tillerd_paths::{
    daemon_socket, daemon_socket_in, data_root, gate_socket_in, manifest_in, resolve_daemon_bin,
    resolve_gate_bin, runtime_dir,
};

pub const ORCHESTRATOR_STATUS_EVENT: &str = "orchestrator://status";

/// The boot lifecycle status as seen on the wire. Unchanged from the prior host.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "orchestrator://status")]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum StatusWire {
    Booting,
    OpeningStore,
    Supervising,
    Ready,
    Failed { reason: String },
}

/// A service's state on the wire. Additive read-only health surface; mirrors
/// `orchestrator::ServiceState`.
#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
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
#[derive(Debug, Clone, Serialize, specta::Type)]
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
}

impl Default for OrchestratorState {
    fn default() -> Self {
        Self {
            status: Arc::new(Mutex::new(StatusWire::Booting)),
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn orchestrator_status(state: State<'_, OrchestratorState>) -> StatusWire {
    state.status.lock().unwrap().clone()
}

/// The manifest path each supervised service writes, kept here so the health read
/// and `build_supervisor` resolve the same locations. Read-only health derives
/// from these manifests; no socket or route is opened.
fn service_health_specs() -> Vec<HealthSpec> {
    let dir = runtime_dir();
    let version = env!("CARGO_PKG_VERSION").to_string();
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

/// Read-only per-service health (gate, daemon), read live from each manifest.
fn service_health_snapshot() -> Vec<ServiceHealthWire> {
    read_service_health(&service_health_specs(), process_launch::pid_is_alive)
        .into_iter()
        .map(ServiceHealthWire::from)
        .collect()
}

#[tauri::command]
#[specta::specta]
pub fn service_health() -> Result<Vec<ServiceHealthWire>, String> {
    Ok(service_health_snapshot())
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
    // Cold-starting a freshly-built service under load (CI / fresh build) can exceed even 30s before
    // the manifest flips to ready; fail-fast on a dead child still keeps a genuine crash from waiting
    // this out, so a generous window only affects the cold path.
    ProcessSupervisor::new()
        .with_timing(SpawnTiming {
            startup_timeout: Duration::from_secs(60),
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

/// The orchestrator core configuration, resolved from the runtime directory.
fn boot_config(app: AppHandle) -> Config {
    Config {
        db_path: data_root().join("domain.db"),
        socket: daemon_socket(),
        fs_root: data_root().join("config"),
        log_dir: runtime_dir(),
        notification_sink: Arc::new(notification::NotificationForwarder::new(app)),
    }
}

fn emit_status<R: tauri::Runtime>(
    app: &AppHandle<R>,
    slot: &Arc<Mutex<StatusWire>>,
    wire: StatusWire,
) {
    *slot.lock().unwrap() = wire.clone();
    let _ = app.emit(ORCHESTRATOR_STATUS_EVENT, wire);
}

/// Build the orchestrator core (bus over the sqlite pool + daemon runtime), supervise
/// the gate/daemon services, then manage the bus so IPC commands can dispatch.
/// Surface streams register their own per-channel sinks via `SubscribeSurface`.
/// Boot phases stream to the renderer over
/// `orchestrator://status`. Runs on a dedicated thread with its own tokio runtime.
pub fn spawn_boot(app: AppHandle, state: &OrchestratorState) {
    let status = state.status.clone();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("build boot runtime");

        emit_status(&app, &status, StatusWire::Booting);

        emit_status(&app, &status, StatusWire::OpeningStore);
        let bus = match runtime.block_on(build_bus(&boot_config(app.clone()))) {
            Ok(bus) => bus,
            Err(error) => {
                // Store open failed: there is no bus to record through, so only the
                // boot-progress status surfaces the failure.
                emit_status(
                    &app,
                    &status,
                    StatusWire::Failed {
                        reason: error.to_string(),
                    },
                );
                eprintln!("orchestrator boot failed (open store): {error}");
                return;
            }
        };

        // Manage the bus immediately so that any early IPC calls from the frontend do not panic.
        app.manage(bus);
        let bus = app.state::<crate::transport::Bus>();

        emit_status(&app, &status, StatusWire::Supervising);
        let mut supervisor = build_supervisor();
        let services = match supervisor.ensure_all() {
            Ok(s) if all_available(&s) => s,
            Ok(_) | Err(_) => {
                let reason = "service not available at boot".to_string();
                emit_status(
                    &app,
                    &status,
                    StatusWire::Failed {
                        reason: reason.clone(),
                    },
                );
                let _ = runtime.block_on(bus.execute_notable(OrchestratorStatus {
                    ready: false,
                    reason: Some(reason.clone()),
                    ts: notification::now_ms(),
                }));
                eprintln!("orchestrator boot failed (supervision): {reason}");
                return;
            }
        };
        let _ = services;

        // Seed prebuilt commands (idempotent) and reconcile surfaces against the daemon.
        let _ = runtime.block_on(bus.execute(SeedCommands));
        if let Err(e) = runtime.block_on(bus.execute(ReconcileSurfaces)) {
            eprintln!("surface reconcile failed (non-fatal): {e}");
        }

        emit_status(&app, &status, StatusWire::Ready);

        let _ = runtime.block_on(bus.execute_notable(OrchestratorStatus {
            ready: true,
            reason: None,
            ts: notification::now_ms(),
        }));

        // Keep the boot runtime alive for the process lifetime so the daemon proxy
        // tasks it spawned continue to run.
        std::mem::forget(runtime);
    });
}

/// Watch the runtime logs directory on a 1s tick, emitting `logs://changed` when the
/// summed size of its `.log` files moves. Poll over a filesystem-notify dependency:
/// the only consumer re-pulls a bounded window, so a 1s nudge is enough and stays
/// self-contained. Runs on a dedicated thread with its own current-thread runtime.
pub fn spawn_logs_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build logs watcher runtime");
        runtime.block_on(async move {
            let dir = tillerd_paths::logging::logs_dir_in(&runtime_dir());
            let mut tick = tokio::time::interval(Duration::from_secs(1));
            let mut last = logs_dir_size(&dir).await;
            loop {
                tick.tick().await;
                let size = logs_dir_size(&dir).await;
                if size != last {
                    last = size;
                    if let Some(bus) = app.try_state::<crate::transport::Bus>() {
                        let event =
                            orchestrator::shared::domain_channel::DomainChannelEvent::Bytes(&[
                                0x00, 110, 117, 108, 108,
                            ]);
                        bus.cx().domain_channel_sinks().dispatch_prefix(
                            "logs://changed/",
                            |sink| {
                                sink.emit(&event);
                            },
                        );
                    }
                }
            }
        });
    });
}

async fn logs_dir_size(dir: &std::path::Path) -> u64 {
    let Ok(mut rd) = tokio::fs::read_dir(dir).await else {
        return 0;
    };
    let mut total = 0u64;
    while let Ok(Some(e)) = rd.next_entry().await {
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some("log") {
            continue;
        }
        total = total.saturating_add(e.metadata().await.map(|m| m.len()).unwrap_or(0));
    }
    total
}
