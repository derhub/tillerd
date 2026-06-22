use std::path::PathBuf;
use std::time::Instant;

use process_launch::Probes;
pub use process_launch::{LaunchError, SpawnTiming};
use service_host::{Manifest, ServiceStatus as Lifecycle};

use crate::shared::{Error, Result};

#[derive(Debug, Clone)]
pub struct ServiceSpec {
    pub name: String,
    pub manifest_path: PathBuf,
    pub socket_path: PathBuf,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    Available,
    Unavailable,
}

#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub name: String,
    pub version: Option<String>,
    pub liveness: Liveness,
    pub pid: Option<u32>,
    pub adopted: bool,
}

impl ServiceStatus {
    pub fn is_available(&self) -> bool {
        self.liveness == Liveness::Available
    }
}

pub fn ensure_service(
    spec: &ServiceSpec,
    timing: &SpawnTiming,
    probes: &impl Probes,
) -> Result<ServiceStatus> {
    if let Some(manifest) = Manifest::read(&spec.manifest_path) {
        if probes.is_alive(manifest.pid) {
            // Readiness comes from the manifest status (ADR-0028), not a socket probe.
            if manifest.version == spec.version && manifest.status == Lifecycle::Ready {
                return Ok(ServiceStatus {
                    name: spec.name.clone(),
                    version: Some(manifest.version),
                    liveness: Liveness::Available,
                    pid: Some(manifest.pid),
                    adopted: true,
                });
            }

            // Live but the wrong version: drain-and-restart (ADR-0029). Signal the old instance to
            // drain (refuse new work, finish active work, exit) and wait for it to release the
            // socket before starting the expected binary. No state handoff between old and new.
            if manifest.version != spec.version {
                probes.drain(manifest.pid);
                wait_for_exit(probes, manifest.pid, timing);
            }
        }
    }

    // A stale socket from a dead/drained instance blocks a clean bind; clear it first.
    probes.remove_socket(&spec.socket_path);

    let pid = probes.spawn().map_err(|e| Error::ServiceUnavailable {
        service: spec.name.clone(),
        reason: e.to_string(),
    })?;

    let deadline = Instant::now() + timing.startup_timeout;
    loop {
        if Manifest::read(&spec.manifest_path).is_some_and(|m| m.status == Lifecycle::Ready) {
            return Ok(ServiceStatus {
                name: spec.name.clone(),
                version: Some(spec.version.clone()),
                liveness: Liveness::Available,
                pid: Some(pid),
                adopted: false,
            });
        }
        // Fail fast if the child exited rather than wait out the whole timeout.
        if !probes.is_alive(pid) {
            return Err(Error::ServiceUnavailable {
                service: spec.name.clone(),
                reason: "process exited during startup".to_string(),
            });
        }
        if Instant::now() >= deadline {
            return Err(Error::ServiceUnavailable {
                service: spec.name.clone(),
                reason: format!(
                    "did not report ready within {} ms",
                    timing.startup_timeout.as_millis()
                ),
            });
        }
        probes.sleep(timing.poll_interval);
    }
}

/// Poll until the draining process exits or the startup window elapses. No force-kill here: an
/// instance with active sessions exits only when it idles or an explicit upgrade-now (SIGTERM)
/// retires it (ADR-0029). If it has not exited by the deadline, the caller proceeds anyway -- the
/// fresh spawn surfaces an unavailable service rather than this blocking forever.
fn wait_for_exit(probes: &impl Probes, pid: u32, timing: &SpawnTiming) {
    let deadline = Instant::now() + timing.startup_timeout;
    while probes.is_alive(pid) {
        if Instant::now() >= deadline {
            break;
        }
        probes.sleep(timing.poll_interval);
    }
}

pub fn all_available(statuses: &[ServiceStatus]) -> bool {
    !statuses.is_empty() && statuses.iter().all(ServiceStatus::is_available)
}

pub trait Supervise {
    fn ensure_all(&mut self) -> Result<Vec<ServiceStatus>>;
}

pub type SpawnFn = Box<dyn Fn() -> std::result::Result<u32, LaunchError>>;

pub struct ProcessSupervisor {
    services: Vec<(ServiceSpec, SpawnFn)>,
    timing: SpawnTiming,
}

impl ProcessSupervisor {
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
            timing: SpawnTiming::default(),
        }
    }

    pub fn with_timing(mut self, timing: SpawnTiming) -> Self {
        self.timing = timing;
        self
    }

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
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    struct FakeProbes {
        alive: bool,
        spawn_result: std::result::Result<u32, LaunchError>,
        spawned: Cell<bool>,
        drained: Cell<bool>,
        // When set, spawn() writes a manifest with this status, simulating the started service.
        on_spawn: Option<(PathBuf, &'static str)>,
    }

    impl FakeProbes {
        fn adoptable() -> Self {
            Self {
                alive: true,
                spawn_result: Ok(999),
                spawned: Cell::new(false),
                drained: Cell::new(false),
                on_spawn: None,
            }
        }
        fn spawnable(manifest_path: PathBuf) -> Self {
            Self {
                alive: false,
                spawn_result: Ok(4242),
                spawned: Cell::new(false),
                drained: Cell::new(false),
                on_spawn: Some((manifest_path, "ready")),
            }
        }
        fn never_ready() -> Self {
            Self {
                alive: true,
                spawn_result: Ok(4242),
                spawned: Cell::new(false),
                drained: Cell::new(false),
                on_spawn: None,
            }
        }
        fn dies_after_spawn() -> Self {
            Self {
                alive: false,
                spawn_result: Ok(4242),
                spawned: Cell::new(false),
                drained: Cell::new(false),
                on_spawn: None,
            }
        }
        /// A live instance serving the wrong version: adoption is refused and it must be drained
        /// before a fresh spawn.
        fn live_mismatch(manifest_path: PathBuf) -> Self {
            Self {
                alive: true,
                spawn_result: Ok(7777),
                spawned: Cell::new(false),
                drained: Cell::new(false),
                on_spawn: Some((manifest_path, "ready")),
            }
        }
    }

    impl Probes for FakeProbes {
        fn is_alive(&self, _pid: u32) -> bool {
            self.alive
        }
        fn is_reachable(&self, _path: &Path) -> bool {
            false
        }
        fn drain(&self, _pid: u32) {
            self.drained.set(true);
        }
        fn remove_socket(&self, _path: &Path) {}
        fn spawn(&self) -> std::result::Result<u32, LaunchError> {
            self.spawned.set(true);
            let pid = self
                .spawn_result
                .as_ref()
                .map(|p| *p)
                .map_err(|e| LaunchError::SpawnFailed(e.to_string()))?;
            if let Some((path, status)) = &self.on_spawn {
                write_manifest(path, pid, "x", status);
            }
            Ok(pid)
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

    fn write_manifest(path: &PathBuf, pid: u32, version: &str, status: &str) {
        std::fs::write(
            path,
            format!(r#"{{"pid":{pid},"version":"{version}","status":"{status}"}}"#),
        )
        .unwrap();
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
        write_manifest(&spec.manifest_path, 4321, "1.2.3", "ready");
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
        let probes = FakeProbes::spawnable(spec.manifest_path.clone());

        let status = ensure_service(&spec, &fast_timing(), &probes).unwrap();

        assert!(!status.adopted);
        assert!(status.is_available());
        assert!(probes.spawned.get(), "an absent service must be spawned");
    }

    #[test]
    fn version_mismatch_falls_through_to_spawn() {
        let (_dir, spec) = temp_spec("mismatch", "2.0.0");
        write_manifest(&spec.manifest_path, 4321, "1.0.0", "ready");
        let probes = FakeProbes::spawnable(spec.manifest_path.clone());

        let status = ensure_service(&spec, &fast_timing(), &probes).unwrap();

        assert!(probes.spawned.get(), "a version mismatch must re-spawn");
        assert!(!status.adopted);
    }

    #[test]
    fn live_version_mismatch_drains_old_then_spawns_fresh() {
        let (_dir, spec) = temp_spec("drain-restart", "2.0.0");
        // A live instance serving the old version is recorded in the manifest.
        write_manifest(&spec.manifest_path, 7777, "1.0.0", "ready");
        let probes = FakeProbes::live_mismatch(spec.manifest_path.clone());

        let status = ensure_service(&spec, &fast_timing(), &probes).unwrap();

        assert!(
            probes.drained.get(),
            "a live version mismatch must drain the old instance before restarting"
        );
        assert!(
            probes.spawned.get(),
            "the expected binary is then spawned fresh"
        );
        assert!(!status.adopted, "the fresh instance is not an adoption");
    }

    #[test]
    fn service_that_never_binds_times_out() {
        let (_dir, spec) = temp_spec("hung", "1.0.0");
        let probes = FakeProbes::never_ready();

        let result = ensure_service(&spec, &fast_timing(), &probes);

        assert!(matches!(
            result,
            Err(Error::ServiceUnavailable { reason, .. }) if reason.contains("did not report ready")
        ));
    }

    #[test]
    fn child_that_exits_during_startup_fails_fast() {
        let (_dir, spec) = temp_spec("dead", "1.0.0");
        // A generous timeout that the fail-fast must short-circuit rather than wait out.
        let timing = SpawnTiming {
            startup_timeout: Duration::from_secs(60),
            poll_interval: Duration::ZERO,
        };
        let probes = FakeProbes::dies_after_spawn();

        let result = ensure_service(&spec, &timing, &probes);

        assert!(matches!(
            result,
            Err(Error::ServiceUnavailable { reason, .. }) if reason.contains("exited during startup")
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
