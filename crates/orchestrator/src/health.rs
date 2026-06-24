//! Read-only per-service health, derived from each service's manifest. Unlike the boot snapshot -- which only exists when
//! every service is available -- this reads live in any state, so a service that
//! is down, mismatched, or draining is observable. It opens no socket and changes
//! no service lifecycle; a pid-liveness check keeps a stale manifest from a
//! crashed service from reading as ready.

use std::path::PathBuf;

use service_host::{Manifest, ServiceStatus as Lifecycle};

/// What to read for one service: its name, where its manifest lives, and the
/// version the host expects it to run.
#[derive(Debug, Clone)]
pub struct HealthSpec {
    /// The service's name (e.g. `gate`, `daemon`).
    pub name: String,
    /// Path to the service's manifest.
    pub manifest_path: PathBuf,
    /// The version the host expects this service to run.
    pub expected_version: String,
}

/// A service's observed state -- richer than boot's available/unavailable so the
/// interface can distinguish a version mismatch or a drain from a healthy service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    /// Manifest present, not yet announced as listening.
    Starting,
    /// Live at the expected version and accepting work.
    Ready,
    /// Winding down: refusing new work, finishing active work before exit.
    Draining,
    /// Live, but running a version other than the one the host expects.
    VersionMismatch,
    /// No manifest, or the recorded process is gone.
    Unavailable,
}

/// One service's observed health.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceHealth {
    pub name: String,
    /// The currently-running version, or `None` when the service is unavailable.
    pub version: Option<String>,
    pub state: ServiceState,
}

/// Read each service's manifest and derive its health. `is_alive` reports whether
/// a recorded pid is still running, so a stale manifest left by a crashed service
/// reads as `Unavailable` rather than its last recorded status.
///
/// Read-only: it opens no socket and changes no service lifecycle.
pub fn read_service_health(
    specs: &[HealthSpec],
    is_alive: impl Fn(u32) -> bool,
) -> Vec<ServiceHealth> {
    specs
        .iter()
        .map(|spec| match Manifest::read(&spec.manifest_path) {
            Some(m) if is_alive(m.pid) => {
                let (state, version) = if m.version != spec.expected_version {
                    (ServiceState::VersionMismatch, Some(m.version))
                } else {
                    let state = match m.status {
                        Lifecycle::Starting => ServiceState::Starting,
                        Lifecycle::Ready => ServiceState::Ready,
                        Lifecycle::Draining => ServiceState::Draining,
                    };
                    (state, Some(m.version))
                };
                ServiceHealth {
                    name: spec.name.clone(),
                    version,
                    state,
                }
            }
            _ => ServiceHealth {
                name: spec.name.clone(),
                version: None,
                state: ServiceState::Unavailable,
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use service_host::{Manifest, ManifestData};
    use std::path::{Path, PathBuf};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orchestrator-health-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn write_manifest(path: &Path, pid: u32, version: &str, status: Lifecycle) {
        Manifest::new(path.to_path_buf())
            .write_data(&ManifestData {
                pid,
                version: version.to_string(),
                status,
                socket_path: None,
            })
            .unwrap();
    }

    fn spec(dir: &Path, name: &str, expected: &str) -> HealthSpec {
        HealthSpec {
            name: name.to_string(),
            manifest_path: dir.join(format!("{name}.json")),
            expected_version: expected.to_string(),
        }
    }

    // Scenario: Each supervised service is reported
    #[test]
    fn each_supervised_service_is_reported_with_name_version_and_state() {
        let dir = temp_dir("reported");
        write_manifest(
            &spec(&dir, "gate", "1.0.0").manifest_path,
            1,
            "1.0.0",
            Lifecycle::Ready,
        );
        write_manifest(
            &spec(&dir, "daemon", "1.0.0").manifest_path,
            2,
            "1.0.0",
            Lifecycle::Ready,
        );
        let specs = vec![spec(&dir, "gate", "1.0.0"), spec(&dir, "daemon", "1.0.0")];

        let health = read_service_health(&specs, |_| true);

        assert_eq!(health.len(), 2);
        assert_eq!(health[0].name, "gate");
        assert_eq!(health[0].version.as_deref(), Some("1.0.0"));
        assert_eq!(health[0].state, ServiceState::Ready);
        assert_eq!(health[1].name, "daemon");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Scenario: Each state is distinguishable (ready / unavailable / starting)
    #[test]
    fn ready_unavailable_and_starting_are_distinguishable() {
        let dir = temp_dir("states");
        write_manifest(
            &spec(&dir, "gate", "1.0.0").manifest_path,
            1,
            "1.0.0",
            Lifecycle::Ready,
        );
        write_manifest(
            &spec(&dir, "daemon", "1.0.0").manifest_path,
            2,
            "1.0.0",
            Lifecycle::Starting,
        );
        // No manifest written for `extra`.
        let specs = vec![
            spec(&dir, "gate", "1.0.0"),
            spec(&dir, "daemon", "1.0.0"),
            spec(&dir, "extra", "1.0.0"),
        ];

        let health = read_service_health(&specs, |_| true);

        assert_eq!(health[0].state, ServiceState::Ready);
        assert_eq!(health[1].state, ServiceState::Starting);
        assert_eq!(health[2].state, ServiceState::Unavailable);
        assert_eq!(health[2].version, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Scenario: Version mismatch and draining are distinct from ready
    #[test]
    fn version_mismatch_and_draining_are_distinct_from_ready() {
        let dir = temp_dir("mismatch-drain");
        write_manifest(
            &spec(&dir, "gate", "2.0.0").manifest_path,
            1,
            "1.0.0",
            Lifecycle::Ready,
        );
        write_manifest(
            &spec(&dir, "daemon", "1.0.0").manifest_path,
            2,
            "1.0.0",
            Lifecycle::Draining,
        );
        let specs = vec![spec(&dir, "gate", "2.0.0"), spec(&dir, "daemon", "1.0.0")];

        let health = read_service_health(&specs, |_| true);

        assert_eq!(health[0].state, ServiceState::VersionMismatch);
        assert_eq!(health[0].version.as_deref(), Some("1.0.0"));
        assert_eq!(health[1].state, ServiceState::Draining);
        assert_ne!(health[0].state, ServiceState::Ready);
        assert_ne!(health[1].state, ServiceState::Ready);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Scenario: No mutating operation is exposed -- a stale manifest from a dead
    // pid reads as unavailable (read path only; the function takes no probe that
    // could start, stop, or restart a service).
    #[test]
    fn stale_manifest_with_dead_pid_is_unavailable() {
        let dir = temp_dir("stale");
        write_manifest(
            &spec(&dir, "gate", "1.0.0").manifest_path,
            99999,
            "1.0.0",
            Lifecycle::Ready,
        );
        let specs = vec![spec(&dir, "gate", "1.0.0")];

        let health = read_service_health(&specs, |_| false);

        assert_eq!(health[0].state, ServiceState::Unavailable);
        assert_eq!(health[0].version, None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
