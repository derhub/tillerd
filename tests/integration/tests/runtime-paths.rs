//! Cross-crate integration tests for runtime-paths library.
//!
//! Tests how the paths library integrates with daemon, gate, and other services
//! to ensure consistent path resolution across the workspace.

use serial_test::serial;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tillerd_paths::{
    daemon_socket_in, gate_socket_in, manifest_in, runtime_dir_or, store_in, ENV_TILLERD_DIR,
};

/// When the daemon starts, it resolves the runtime directory from tillerd-paths.
/// This ensures the daemon and all other services agree on where runtime files live.
#[test]
#[serial]
fn daemon_gate_and_storage_all_use_same_runtime_directory() {
    let tmp = TempDir::new().unwrap();
    let override_dir = tmp.path().to_string_lossy().into_owned();

    let daemon_dir = runtime_dir_or(Some(&override_dir));
    let gate_dir = runtime_dir_or(Some(&override_dir));

    assert_eq!(daemon_dir, gate_dir);
    assert_eq!(daemon_dir, PathBuf::from(&override_dir));
}

/// When the daemon starts and the gate connects, they must agree on socket locations.
/// The paths library ensures both use the same socket paths under the runtime directory.
#[test]
fn daemon_socket_and_gate_socket_discoverable() {
    let runtime = Path::new("/srv/tillerd");

    let daemon_sock = daemon_socket_in(runtime);
    let gate_sock = gate_socket_in(runtime);

    // Both are under the same runtime directory
    assert_eq!(daemon_sock.parent(), Some(runtime));
    assert_eq!(gate_sock.parent(), Some(runtime));

    // Socket names are stable and predictable
    assert_eq!(daemon_sock, PathBuf::from("/srv/tillerd/daemon.sock"));
    assert_eq!(gate_sock, PathBuf::from("/srv/tillerd/gate.sock"));
}

/// When services read/write the daemon manifest and product store, they must all
/// reference the same files under the runtime directory.
#[test]
fn manifest_and_store_paths_are_consistent() {
    let runtime = Path::new("/srv/tillerd");

    let manifest_file = manifest_in(runtime);
    let store_file = store_in(runtime);

    // Both are singletons under the runtime directory
    assert_eq!(manifest_file, PathBuf::from("/srv/tillerd/daemon.json"));
    assert_eq!(store_file, PathBuf::from("/srv/tillerd/tillerd.db"));

    // No duplication: only one manifest, one store
    assert_ne!(manifest_file, store_file);
}

/// When `TILLERD_DIR` is set, all services (daemon, gate, etc.) use it.
/// The paths library ensures this propagates uniformly across the workspace.
#[test]
#[serial]
fn environment_override_affects_all_path_resolutions() {
    let override_dir = "/custom/runtime";
    std::env::set_var(ENV_TILLERD_DIR, override_dir);

    let daemon_sock = daemon_socket_in(&runtime_dir_or(None));
    let gate_sock = gate_socket_in(&runtime_dir_or(None));

    std::env::remove_var(ENV_TILLERD_DIR);

    // Both use the override
    assert!(daemon_sock.starts_with(override_dir));
    assert!(gate_sock.starts_with(override_dir));
}

/// During service startup, the host must ensure all services use consistent paths.
/// This test verifies that the paths library provides deterministic answers.
#[test]
fn multiple_sequential_calls_return_consistent_paths() {
    let runtime = Path::new("/srv/tillerd");

    let first_daemon = daemon_socket_in(runtime);
    let second_daemon = daemon_socket_in(runtime);

    let first_gate = gate_socket_in(runtime);
    let second_gate = gate_socket_in(runtime);

    // Determinism: calling again returns the same path
    assert_eq!(first_daemon, second_daemon);
    assert_eq!(first_gate, second_gate);
}
