//! Adopt-or-spawn orchestration coverage using mock boundary I/O.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::time::Duration;

use process_launch::{
    adopt_or_spawn, manifest, spawn_and_wait, LaunchError, Launched, Probes, SpawnTiming,
};

struct FakeProbes {
    alive: bool,
    reachable: bool,
    spawned: Cell<bool>,
    spawn_pid: u32,
}

impl FakeProbes {
    fn new(alive: bool, reachable: bool, spawn_pid: u32) -> Self {
        Self {
            alive,
            reachable,
            spawned: Cell::new(false),
            spawn_pid,
        }
    }
}

impl Probes for FakeProbes {
    fn is_alive(&self, _pid: u32) -> bool {
        self.alive
    }
    fn is_reachable(&self, _path: &Path) -> bool {
        // After a spawn the socket is reachable; before a spawn it follows the flag.
        self.reachable || self.spawned.get()
    }
    fn remove_socket(&self, _path: &Path) {}
    fn spawn(&self) -> Result<u32, LaunchError> {
        self.spawned.set(true);
        Ok(self.spawn_pid)
    }
    fn sleep(&self, _dur: Duration) {}
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "process-launch-e2e-{}-{}-{tag}",
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
fn adopt_a_live_matching_instance_does_not_start_a_second() {
    let dir = temp_dir("adopt");
    manifest::write(&dir, 4321, "1.0.0").unwrap();
    let probes = FakeProbes::new(true, true, 9999);

    let launched = adopt_or_spawn(&dir, "1.0.0", &fast_timing(), &probes).unwrap();

    assert_eq!(launched, Launched::Adopted { pid: 4321 });
    assert!(!probes.spawned.get());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn spawn_when_none_is_running_returns_only_once_reachable() {
    let dir = temp_dir("spawn-none");
    let probes = FakeProbes::new(false, false, 7777);

    let launched = adopt_or_spawn(&dir, "1.0.0", &fast_timing(), &probes).unwrap();

    assert_eq!(launched, Launched::Spawned { pid: 7777 });
    assert!(probes.spawned.get());
    let m = manifest::read(&dir).unwrap();
    assert_eq!(m.pid, 7777);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stale_manifest_naming_dead_pid_is_overwritten_by_spawn() {
    let dir = temp_dir("stale");
    manifest::write(&dir, 111, "0.0.1").unwrap();
    let probes = FakeProbes::new(false, false, 8888);

    let launched = adopt_or_spawn(&dir, "2.0.0", &fast_timing(), &probes).unwrap();

    assert_eq!(launched, Launched::Spawned { pid: 8888 });
    let m = manifest::read(&dir).unwrap();
    assert_eq!(m.pid, 8888);
    assert_eq!(m.version, "2.0.0");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn spawn_and_wait_is_reexported_at_crate_root() {
    let dir = temp_dir("reexport");
    let probes = FakeProbes::new(false, false, 6666);
    let pid = spawn_and_wait(&dir, "1.0.0", &fast_timing(), &probes).unwrap();
    assert_eq!(pid, 6666);
    let _ = std::fs::remove_dir_all(&dir);
}
