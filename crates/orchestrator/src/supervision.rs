//! Adopt-or-spawn supervision of the shared services (gate, daemon) and the
//! readiness gate over them.
//!
//! Supervision composes [`process_launch`]: liveness is a control-socket connect
//! and version is read from the service's manifest — exactly the service
//! contract. Per ADR-0019 there is no health socket; each service self-checks
//! in-process, and the orchestrator observes availability through the control
//! socket. `process_launch::adopt_or_spawn` is keyed to the daemon's fixed file
//! names, so [`ensure_service`] generalizes the same adopt-or-spawn decision over
//! an explicit manifest/socket path per service, letting one routine supervise
//! both the daemon (`daemon.json`/`daemon.sock`) and the gate
//! (`gate.json`/`gate.sock`).

use std::path::{Path, PathBuf};
use std::time::Instant;

use process_launch::{LaunchError, ManifestData, Probes, SpawnTiming};

use crate::error::{OrchestratorError, Result};

/// Identity and on-disk locations of a supervised service.
#[derive(Debug, Clone)]
pub struct ServiceSpec {
    /// The service name, e.g. `gate` or `daemon`.
    pub name: String,
    /// Path to the service's manifest (`{pid, version}`).
    pub manifest_path: PathBuf,
    /// Path to the service's control socket; a successful connect is liveness.
    pub socket_path: PathBuf,
    /// The version required for adoption (an exact match adopts; otherwise spawn).
    pub version: String,
}

/// Whether a supervised service is reachable on its control socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    /// The service is reachable and serving.
    Available,
    /// The service is not reachable.
    Unavailable,
}

/// The tracked status of a supervised service: its liveness and version, sourced
/// from a control-socket connect and the service manifest (ADR-0019 — no health
/// socket).
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    /// The service name.
    pub name: String,
    /// The service version from its manifest, when known.
    pub version: Option<String>,
    /// Whether the service is reachable on its control socket.
    pub liveness: Liveness,
    /// The service pid, when known.
    pub pid: Option<u32>,
    /// Whether a running instance was adopted (vs freshly spawned).
    pub adopted: bool,
}

impl ServiceStatus {
    /// Whether the service is available (reachable).
    pub fn is_available(&self) -> bool {
        self.liveness == Liveness::Available
    }
}

/// Read a `{pid, version}` manifest at an arbitrary path, or `None` if absent or
/// malformed. Generalizes `process_launch::manifest::read`, which is keyed to the
/// daemon's fixed file name.
fn read_manifest(path: &Path) -> Option<ManifestData> {
    let raw = std::fs::read(path).ok()?;
    serde_json::from_slice(&raw).ok()
}

/// Adopt a live, compatible instance of `spec`, or spawn one and wait until it is
/// reachable on its control socket.
///
/// Adoption requires a manifest naming a live pid at an exact version match whose
/// control socket accepts a connection. Any other state falls through to spawn. A
/// service that cannot be made reachable within the startup window yields a typed
/// [`OrchestratorError::ServiceUnavailable`].
pub fn ensure_service(
    spec: &ServiceSpec,
    timing: &SpawnTiming,
    probes: &impl Probes,
) -> Result<ServiceStatus> {
    if let Some(manifest) = read_manifest(&spec.manifest_path) {
        if probes.is_alive(manifest.pid)
            && manifest.version == spec.version
            && probes.is_reachable(&spec.socket_path)
        {
            return Ok(ServiceStatus {
                name: spec.name.clone(),
                version: Some(manifest.version),
                liveness: Liveness::Available,
                pid: Some(manifest.pid),
                adopted: true,
            });
        }
    }

    // A stale socket from a dead instance blocks a clean bind; clear it first.
    probes.remove_socket(&spec.socket_path);

    let pid = probes
        .spawn()
        .map_err(|e| OrchestratorError::ServiceUnavailable {
            service: spec.name.clone(),
            reason: e.to_string(),
        })?;

    let deadline = Instant::now() + timing.startup_timeout;
    loop {
        if probes.is_reachable(&spec.socket_path) {
            return Ok(ServiceStatus {
                name: spec.name.clone(),
                version: Some(spec.version.clone()),
                liveness: Liveness::Available,
                pid: Some(pid),
                adopted: false,
            });
        }
        if Instant::now() >= deadline {
            return Err(OrchestratorError::ServiceUnavailable {
                service: spec.name.clone(),
                reason: format!(
                    "did not become reachable within {} ms",
                    timing.startup_timeout.as_millis()
                ),
            });
        }
        probes.sleep(timing.poll_interval);
    }
}

/// Readiness over a set of supervised services: every service must be available.
/// An empty set is not ready — readiness requires the supervised services to
/// actually be present (orchestrator-supervision: readiness gated on services).
pub fn all_available(statuses: &[ServiceStatus]) -> bool {
    !statuses.is_empty() && statuses.iter().all(ServiceStatus::is_available)
}

/// Ensures every required service is available at boot, surfacing a typed failure
/// if any cannot be. Boot depends on this trait so it can be faked in tests.
pub trait Supervise {
    /// Adopt-or-spawn every required service and return their statuses, or a
    /// typed failure if one cannot be made available.
    fn ensure_all(&mut self) -> Result<Vec<ServiceStatus>>;
}

/// How to launch a service when it must be spawned. The host supplies the
/// resolved binary and environment behind this closure, keeping binary
/// resolution out of the runtime-agnostic crate.
pub type SpawnFn = Box<dyn Fn() -> std::result::Result<u32, LaunchError>>;

/// The production supervisor: adopt-or-spawns each configured service through the
/// OS via `process_launch::OsProbes`.
pub struct ProcessSupervisor {
    services: Vec<(ServiceSpec, SpawnFn)>,
    timing: SpawnTiming,
}

impl ProcessSupervisor {
    /// An empty supervisor with default spawn timing.
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
            timing: SpawnTiming::default(),
        }
    }

    /// Override the spawn timing applied to every service.
    pub fn with_timing(mut self, timing: SpawnTiming) -> Self {
        self.timing = timing;
        self
    }

    /// Register a service and how to spawn it. Services are ensured in
    /// registration order.
    pub fn service(mut self, spec: ServiceSpec, spawn: SpawnFn) -> Self {
        self.services.push((spec, spawn));
        self
    }
}

impl Default for ProcessSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Supervise for ProcessSupervisor {
    fn ensure_all(&mut self) -> Result<Vec<ServiceStatus>> {
        let mut statuses = Vec::with_capacity(self.services.len());
        for (spec, spawn) in &self.services {
            let probes = process_launch::OsProbes::new(spawn);
            statuses.push(ensure_service(spec, &self.timing, &probes)?);
        }
        Ok(statuses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::path::PathBuf;
    use std::time::Duration;

    struct FakeProbes {
        alive: bool,
        reachable: bool,
        spawn_result: std::result::Result<u32, LaunchError>,
        spawned: Cell<bool>,
    }

    impl FakeProbes {
        fn adoptable() -> Self {
            Self {
                alive: true,
                reachable: true,
                spawn_result: Ok(999),
                spawned: Cell::new(false),
            }
        }
        fn spawnable() -> Self {
            Self {
                alive: false,
                reachable: true,
                spawn_result: Ok(4242),
                spawned: Cell::new(false),
            }
        }
        fn never_reachable() -> Self {
            Self {
                alive: false,
                reachable: false,
                spawn_result: Ok(4242),
                spawned: Cell::new(false),
            }
        }
    }

    impl Probes for FakeProbes {
        fn is_alive(&self, _pid: u32) -> bool {
            self.alive
        }
        fn is_reachable(&self, _path: &Path) -> bool {
            self.reachable
        }
        fn remove_socket(&self, _path: &Path) {}
        fn spawn(&self) -> std::result::Result<u32, LaunchError> {
            self.spawned.set(true);
            self.spawn_result
                .as_ref()
                .map(|p| *p)
                .map_err(|e| LaunchError::SpawnFailed(e.to_string()))
        }
        fn sleep(&self, _dur: Duration) {}
    }

    fn temp_spec(tag: &str, version: &str) -> (tempfile::TempDir, ServiceSpec) {
        let dir = tempfile::tempdir().unwrap();
        let spec = ServiceSpec {
            name: format!("svc-{tag}"),
            manifest_path: dir.path().join("svc.json"),
            socket_path: dir.path().join("svc.sock"),
            version: version.to_string(),
        };
        (dir, spec)
    }

    fn write_manifest(path: &PathBuf, pid: u32, version: &str) {
        std::fs::write(path, format!(r#"{{"pid":{pid},"version":"{version}"}}"#)).unwrap();
    }

    fn fast_timing() -> SpawnTiming {
        SpawnTiming {
            startup_timeout: Duration::ZERO,
            poll_interval: Duration::ZERO,
        }
    }

    #[test]
    fn adopts_live_compatible_service_without_spawning() {
        let (_dir, spec) = temp_spec("adopt", "1.2.3");
        write_manifest(&spec.manifest_path, 4321, "1.2.3");
        let probes = FakeProbes::adoptable();

        let status = ensure_service(&spec, &fast_timing(), &probes).unwrap();

        assert!(status.adopted);
        assert!(status.is_available());
        assert_eq!(status.version.as_deref(), Some("1.2.3"));
        assert!(!probes.spawned.get(), "must not spawn a duplicate");
    }

    #[test]
    fn spawns_absent_service() {
        let (_dir, spec) = temp_spec("spawn", "1.0.0");
        // No manifest written: the service is absent.
        let probes = FakeProbes::spawnable();

        let status = ensure_service(&spec, &fast_timing(), &probes).unwrap();

        assert!(!status.adopted);
        assert!(status.is_available());
        assert!(probes.spawned.get(), "an absent service must be spawned");
    }

    #[test]
    fn version_mismatch_falls_through_to_spawn() {
        let (_dir, spec) = temp_spec("mismatch", "2.0.0");
        write_manifest(&spec.manifest_path, 4321, "1.0.0");
        let probes = FakeProbes::spawnable();

        let status = ensure_service(&spec, &fast_timing(), &probes).unwrap();

        assert!(probes.spawned.get(), "a version mismatch must re-spawn");
        assert!(!status.adopted);
    }

    #[test]
    fn service_that_cannot_be_made_available_is_a_typed_failure() {
        let (_dir, spec) = temp_spec("dead", "1.0.0");
        let probes = FakeProbes::never_reachable();

        let result = ensure_service(&spec, &fast_timing(), &probes);

        assert!(matches!(
            result,
            Err(OrchestratorError::ServiceUnavailable { .. })
        ));
    }

    #[test]
    fn readiness_requires_every_service_available() {
        let available = ServiceStatus {
            name: "gate".into(),
            version: Some("1".into()),
            liveness: Liveness::Available,
            pid: Some(1),
            adopted: true,
        };
        let unavailable = ServiceStatus {
            name: "daemon".into(),
            version: None,
            liveness: Liveness::Unavailable,
            pid: None,
            adopted: false,
        };

        assert!(all_available(std::slice::from_ref(&available)));
        assert!(!all_available(&[available, unavailable]));
        assert!(!all_available(&[]), "no services is not ready");
    }
}
