//! Adopt: exact-version match required. Otherwise spawn.

use std::path::Path;

use crate::manifest::{self};
use crate::probes::Probes;

/// Why adoption did not happen, so the spawn path knows how to proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdoptMiss {
    /// No manifest was present.
    NoManifest,
    /// The manifest named a process that is no longer alive (stale manifest).
    DeadPid {
        /// The dead pid recorded in the manifest.
        pid: u32,
    },
    /// The manifest version does not match the requested version (R3).
    VersionMismatch {
        /// Version recorded in the live instance's manifest.
        running: String,
        /// Version the caller requires.
        wanted: String,
    },
    /// The process is alive but its control socket did not accept a connection.
    SocketUnresponsive,
}

/// Outcome of an adoption attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adoption {
    /// A live matching instance was found and is reachable; connect to it.
    Adopted {
        /// Pid of the adopted instance.
        pid: u32,
    },
    /// No instance was adopted; the caller should spawn for the stated reason.
    Spawn(AdoptMiss),
}

/// Decide whether to adopt the instance described by the manifest under `dir`.
///
/// Order matters: a stale manifest naming a dead pid yields `DeadPid` so the
/// spawn path overwrites it; a version mismatch on a live instance yields
/// `VersionMismatch` so the spawn path replaces it.
pub fn evaluate(dir: &Path, wanted_version: &str, probes: &impl Probes) -> Adoption {
    let Some(m) = manifest::read(dir) else {
        return Adoption::Spawn(AdoptMiss::NoManifest);
    };

    if !probes.is_alive(m.pid) {
        return Adoption::Spawn(AdoptMiss::DeadPid { pid: m.pid });
    }

    if m.version != wanted_version {
        return Adoption::Spawn(AdoptMiss::VersionMismatch {
            running: m.version,
            wanted: wanted_version.to_string(),
        });
    }

    if !probes.is_reachable(&tillerd_paths::daemon_socket_in(dir)) {
        return Adoption::Spawn(AdoptMiss::SocketUnresponsive);
    }

    Adoption::Adopted { pid: m.pid }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::LaunchError;
    use std::cell::Cell;
    use std::path::PathBuf;
    use std::time::Duration;

    struct FakeProbes {
        alive: bool,
        reachable: bool,
        spawned: Cell<bool>,
    }

    impl FakeProbes {
        fn new(alive: bool, reachable: bool) -> Self {
            Self {
                alive,
                reachable,
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
        fn spawn(&self) -> Result<u32, LaunchError> {
            self.spawned.set(true);
            Ok(999)
        }
        fn sleep(&self, _dur: Duration) {}
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "process-launch-adopt-{}-{}-{tag}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn adopt_does_not_spawn_second_instance_when_live_matching_exists() {
        let dir = temp_dir("live-match");
        manifest::write(&dir, 4321, "1.2.3").unwrap();
        let probes = FakeProbes::new(true, true);

        let outcome = evaluate(&dir, "1.2.3", &probes);

        assert_eq!(outcome, Adoption::Adopted { pid: 4321 });
        assert!(!probes.spawned.get());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn adopt_connects_to_existing_socket_without_restart() {
        let dir = temp_dir("socket");
        manifest::write(&dir, 4321, "1.2.3").unwrap();
        let probes = FakeProbes::new(true, true);

        // A reachable socket on a live matching instance is adopted directly.
        assert!(matches!(
            evaluate(&dir, "1.2.3", &probes),
            Adoption::Adopted { .. }
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn adopt_validates_matching_version_before_connecting() {
        let dir = temp_dir("version");
        manifest::write(&dir, 4321, "1.2.3").unwrap();
        let probes = FakeProbes::new(true, true);

        let outcome = evaluate(&dir, "9.9.9", &probes);

        assert_eq!(
            outcome,
            Adoption::Spawn(AdoptMiss::VersionMismatch {
                running: "1.2.3".into(),
                wanted: "9.9.9".into(),
            })
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
