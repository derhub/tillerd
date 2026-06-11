//! Spawn and wait for control socket reachability. Manifest rewritten with new pid.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::error::LaunchError;
use crate::manifest;
use crate::probes::Probes;

/// Timing for a spawn attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnTiming {
    /// Total time to wait for the control socket to become reachable.
    pub startup_timeout: Duration,
    /// Interval between reachability polls.
    pub poll_interval: Duration,
}

impl Default for SpawnTiming {
    fn default() -> Self {
        Self {
            startup_timeout: Duration::from_millis(10_000),
            poll_interval: Duration::from_millis(100),
        }
    }
}

/// Spawn the backend, wait until reachable, and record the new pid in the manifest.
///
/// Removes any pre-existing socket before spawning so the child binds cleanly.
/// On reachability the manifest under `dir` is overwritten with the spawned pid
/// and `version`. Returns [`LaunchError::Timeout`] if the socket never becomes
/// reachable within the configured window.
pub fn spawn_and_wait(
    dir: &Path,
    version: &str,
    timing: &SpawnTiming,
    probes: &impl Probes,
) -> Result<u32, LaunchError> {
    let sock = tillerd_paths::daemon_socket_in(dir);
    probes.remove_socket(&sock);

    let pid = probes.spawn()?;

    let deadline = Instant::now() + timing.startup_timeout;
    loop {
        if probes.is_reachable(&sock) {
            manifest::write(dir, pid, version)
                .map_err(|e| LaunchError::SpawnFailed(e.to_string()))?;
            return Ok(pid);
        }
        if Instant::now() >= deadline {
            return Err(LaunchError::Timeout(
                timing.startup_timeout.as_millis() as u64
            ));
        }
        probes.sleep(timing.poll_interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::path::PathBuf;

    struct FakeProbes {
        spawn_pid: u32,
        reachable_after: u32,
        polls: Cell<u32>,
        removed: Cell<bool>,
        spawned: Cell<bool>,
        spawn_err: bool,
    }

    impl FakeProbes {
        fn reachable_after(n: u32) -> Self {
            Self {
                spawn_pid: 5000,
                reachable_after: n,
                polls: Cell::new(0),
                removed: Cell::new(false),
                spawned: Cell::new(false),
                spawn_err: false,
            }
        }
        fn never_reachable() -> Self {
            Self {
                spawn_pid: 5000,
                reachable_after: u32::MAX,
                polls: Cell::new(0),
                removed: Cell::new(false),
                spawned: Cell::new(false),
                spawn_err: false,
            }
        }
    }

    impl Probes for FakeProbes {
        fn is_alive(&self, _pid: u32) -> bool {
            false
        }
        fn is_reachable(&self, _path: &Path) -> bool {
            self.polls.get() >= self.reachable_after
        }
        fn remove_socket(&self, _path: &Path) {
            self.removed.set(true);
        }
        fn spawn(&self) -> Result<u32, LaunchError> {
            if self.spawn_err {
                return Err(LaunchError::SpawnFailed("boom".into()));
            }
            self.spawned.set(true);
            Ok(self.spawn_pid)
        }
        fn sleep(&self, _dur: Duration) {
            self.polls.set(self.polls.get() + 1);
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "process-launch-spawn-{}-{}-{tag}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn fast_timing() -> SpawnTiming {
        SpawnTiming {
            startup_timeout: Duration::from_millis(500),
            poll_interval: Duration::from_millis(1),
        }
    }

    #[test]
    fn spawn_launches_new_process_when_socket_missing() {
        let dir = temp_dir("launch");
        let probes = FakeProbes::reachable_after(0);

        let pid = spawn_and_wait(&dir, "1.0.0", &fast_timing(), &probes).unwrap();

        assert_eq!(pid, 5000);
        assert!(probes.spawned.get());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn spawn_waits_until_socket_reachable() {
        let dir = temp_dir("wait");
        let probes = FakeProbes::reachable_after(3);

        let pid = spawn_and_wait(&dir, "1.0.0", &fast_timing(), &probes).unwrap();

        assert_eq!(pid, 5000);
        assert!(probes.polls.get() >= 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn spawn_honors_startup_timeout_on_unresponsive_binary() {
        let dir = temp_dir("timeout");
        let probes = FakeProbes::never_reachable();
        let timing = SpawnTiming {
            startup_timeout: Duration::from_millis(10),
            poll_interval: Duration::from_millis(1),
        };

        let err = spawn_and_wait(&dir, "1.0.0", &timing, &probes).unwrap_err();

        assert!(matches!(err, LaunchError::Timeout(10)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn spawn_overwrites_stale_manifest_on_dead_pid() {
        let dir = temp_dir("stale-overwrite");
        manifest::write(&dir, 111, "0.0.1").unwrap();
        let probes = FakeProbes::reachable_after(0);

        spawn_and_wait(&dir, "2.0.0", &fast_timing(), &probes).unwrap();

        let m = manifest::read(&dir).unwrap();
        assert_eq!(m.pid, 5000);
        assert_eq!(m.version, "2.0.0");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn spawn_cleans_dead_socket_before_binding() {
        let dir = temp_dir("clean-socket");
        let probes = FakeProbes::reachable_after(0);

        spawn_and_wait(&dir, "1.0.0", &fast_timing(), &probes).unwrap();

        assert!(probes.removed.get());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn spawn_proceeds_despite_stale_manifest_pid() {
        let dir = temp_dir("stale-proceed");
        manifest::write(&dir, 222, "0.0.9").unwrap();
        let probes = FakeProbes::reachable_after(1);

        let pid = spawn_and_wait(&dir, "3.0.0", &fast_timing(), &probes).unwrap();

        assert_eq!(pid, 5000);
        assert!(probes.spawned.get());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
