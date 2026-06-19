use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use orchestrator::persistence::{CompositeStore, ProjectId, Store};
use orchestrator::supervision::{
    LaunchError, ProcessSupervisor, ServiceSpec, SpawnFn, SpawnTiming,
};
use orchestrator::{boot, EventSink, Status};

struct NullSink;
impl EventSink for NullSink {
    fn emit(&self, _event: &Status) {}
}

fn target_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

fn binaries() -> Option<(PathBuf, PathBuf)> {
    let dir = target_dir();
    let gate = dir.join("tillerd-gate");
    let daemon = dir.join("tillerd-daemon");
    (gate.exists() && daemon.exists()).then_some((gate, daemon))
}

fn spawn_fn(bin: PathBuf, dir: PathBuf) -> SpawnFn {
    Box::new(move || {
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

fn supervisor(dir: &Path, gate: &Path, daemon: &Path) -> ProcessSupervisor {
    let version = env!("CARGO_PKG_VERSION").to_string();
    ProcessSupervisor::new()
        .with_timing(SpawnTiming {
            startup_timeout: Duration::from_secs(15),
            poll_interval: Duration::from_millis(100),
        })
        .service(
            ServiceSpec {
                name: "gate".to_string(),
                manifest_path: dir.join("gate.json"),
                socket_path: dir.join("gate.sock"),
                version: version.clone(),
            },
            spawn_fn(gate.to_path_buf(), dir.to_path_buf()),
        )
        .service(
            ServiceSpec {
                name: "daemon".to_string(),
                manifest_path: dir.join("daemon.json"),
                socket_path: dir.join("daemon.sock"),
                version,
            },
            spawn_fn(daemon.to_path_buf(), dir.to_path_buf()),
        )
}

fn kill(pid: u32) {
    let _ = Command::new("kill").arg(pid.to_string()).status();
}

#[test]
#[ignore = "spawns the real gate and daemon; run with --ignored after building both binaries"]
fn cold_boot_spawns_services_then_reboot_adopts_them() {
    let Some((gate, daemon)) = binaries() else {
        eprintln!(
            "skip: tillerd-gate / tillerd-daemon not found in {:?}",
            target_dir()
        );
        return;
    };

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let store_path = dir.join("tillerd.db");
    let data_root = dir.join("data");

    let mut sup = supervisor(dir, &gate, &daemon);
    let orch = boot(
        || {
            CompositeStore::open(data_root.clone(), store_path.clone())
                .map(|s| Box::new(s) as Box<dyn Store>)
        },
        &mut sup,
        &NullSink,
    )
    .expect("cold boot reaches ready");

    assert!(orch.is_ready());
    assert!(store_path.exists(), "a fresh tillerd.db is created on boot");
    assert!(
        orch.store()
            .get_project(&ProjectId::unfiled())
            .unwrap()
            .is_some(),
        "the Unfiled project is seeded"
    );
    let cold = orch.service_statuses();
    assert_eq!(cold.len(), 2);
    assert!(
        cold.iter().all(|s| s.is_available()),
        "both services available"
    );
    assert!(
        cold.iter().all(|s| !s.adopted),
        "a cold boot spawns, it does not adopt"
    );
    let spawned_pids: Vec<u32> = cold.iter().filter_map(|s| s.pid).collect();

    let mut sup2 = supervisor(dir, &gate, &daemon);
    let orch2 = boot(
        || {
            CompositeStore::open(data_root.clone(), store_path.clone())
                .map(|s| Box::new(s) as Box<dyn Store>)
        },
        &mut sup2,
        &NullSink,
    )
    .expect("re-boot reaches ready");

    assert!(orch2.is_ready());
    let warm = orch2.service_statuses();
    assert!(
        warm.iter().all(|s| s.adopted),
        "a re-boot adopts the running services"
    );

    let mut adopted_pids: Vec<u32> = warm.iter().filter_map(|s| s.pid).collect();
    let mut expected = spawned_pids.clone();
    adopted_pids.sort_unstable();
    expected.sort_unstable();
    assert_eq!(
        adopted_pids, expected,
        "adopted pids match the spawned ones — no duplicate spawn"
    );

    for pid in spawned_pids {
        kill(pid);
    }
}
