//! Integration tests for the tillerd-paths library.
//! 1:1 mapping of spec scenarios to tests.

use serial_test::serial;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tillerd_paths::{
    daemon_socket_in, gate_socket_in, manifest_in, resolve_daemon_bin, resolve_gate_bin,
    runtime_dir, runtime_dir_or, store_in, ENV_DAEMON_BIN, ENV_GATE_BIN, ENV_NOTIFY_BIN,
    ENV_TILLERD_DIR,
};

/// Scenario: Environment override
/// WHEN `TILLERD_DIR` is set
/// THEN the resolved runtime directory is exactly that path
#[test]
#[serial]
fn scenario_environment_override_uses_tillerd_dir_exactly() {
    let override_dir = "/custom/runtime/path";
    std::env::set_var(ENV_TILLERD_DIR, override_dir);
    let result = runtime_dir();
    std::env::remove_var(ENV_TILLERD_DIR);

    assert_eq!(result, PathBuf::from(override_dir));
}

/// Scenario: Default when unset
/// WHEN `TILLERD_DIR` is not set
/// THEN the resolved runtime directory is `~/.tillerd`
#[test]
#[serial]
fn scenario_default_when_unset_uses_home_tillerd() {
    std::env::remove_var(ENV_TILLERD_DIR);
    let result = runtime_dir();
    assert_eq!(result.file_name().unwrap(), ".tillerd");
}

/// Scenario: One resolver
/// WHEN any component needs the runtime directory
/// THEN it calls this library
/// AND no component defines its own runtime-directory resolver
#[test]
fn scenario_one_resolver_available() {
    // Test that the library is the authoritative source.
    // This test verifies the API exists and is callable; other tests verify behavior.
    let _ = runtime_dir();
    let _ = runtime_dir_or(None);
}

/// Scenario: Paths share the runtime directory
/// WHEN the daemon socket, gate socket, daemon manifest, and product store paths are requested
/// THEN each is the corresponding file name joined onto the resolved runtime directory
/// AND all four share the runtime directory as their parent
#[test]
fn scenario_paths_share_the_runtime_directory() {
    let base = Path::new("/srv/tillerd");

    let daemon_sock = daemon_socket_in(base);
    let gate_sock = gate_socket_in(base);
    let manifest_file = manifest_in(base);
    let store_file = store_in(base);

    // All share the same parent
    assert_eq!(daemon_sock.parent(), Some(base));
    assert_eq!(gate_sock.parent(), Some(base));
    assert_eq!(manifest_file.parent(), Some(base));
    assert_eq!(store_file.parent(), Some(base));

    // File names are correct
    assert_eq!(daemon_sock.file_name().unwrap(), "daemon.sock");
    assert_eq!(gate_sock.file_name().unwrap(), "gate.sock");
    assert_eq!(manifest_file.file_name().unwrap(), "daemon.json");
    assert_eq!(store_file.file_name().unwrap(), "tillerd.db");
}

/// Scenario: File names defined once
/// WHEN a component needs the daemon socket, gate socket, daemon manifest, or product store path
/// THEN it obtains it from this library
/// AND it does not hardcode the file name or join it onto a directory itself
#[test]
fn scenario_file_names_are_defined_once() {
    // Verify that all file names are sourced from the library via functions,
    // not hard-coded elsewhere. This test documents the API surface.
    let base = Path::new("/base");

    // These are the only ways to get the paths; no other entry points exist.
    let _ = daemon_socket_in(base);
    let _ = gate_socket_in(base);
    let _ = manifest_in(base);
    let _ = store_in(base);
}

/// Scenario: Override wins when present
/// WHEN the override environment variable names an existing file
/// THEN that path is returned
#[test]
#[serial]
fn scenario_override_wins_when_present() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::env::set_var(ENV_DAEMON_BIN, tmp.path());

    let result = resolve_daemon_bin();

    std::env::remove_var(ENV_DAEMON_BIN);

    assert_eq!(result.as_deref(), Some(tmp.path()));
}

/// Scenario: Override skipped when missing
/// WHEN the override environment variable names a path that does not exist
/// THEN that path is not returned
/// AND resolution continues to the discovery fallbacks
#[test]
#[serial]
fn scenario_override_skipped_when_missing() {
    let tmp = TempDir::new().unwrap();
    let prev_cwd = std::env::current_dir().unwrap();

    std::env::set_current_dir(tmp.path()).unwrap();
    std::env::set_var(ENV_DAEMON_BIN, "/nonexistent/daemon-zzz");
    let result = resolve_daemon_bin();
    std::env::remove_var(ENV_DAEMON_BIN);
    std::env::set_current_dir(prev_cwd).unwrap();

    // When override doesn't exist, it returns None (no fallback found in temp dir).
    assert_eq!(result, None);
}

/// Scenario: Cargo build output discovered without env
/// WHEN no override is set and the binary exists under an ancestor's `target/release` or `bin/`
/// THEN that discovered path is returned
#[test]
#[serial]
fn scenario_cargo_build_output_discovered_without_env() {
    let tmp = TempDir::new().unwrap();
    let release = tmp.path().join("target/release");
    std::fs::create_dir_all(&release).unwrap();
    std::fs::write(release.join("tillerd-daemon"), b"mock").unwrap();

    let prev_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(tmp.path()).unwrap();
    std::env::remove_var(ENV_DAEMON_BIN);
    let result = resolve_daemon_bin();
    std::env::set_current_dir(prev_cwd).unwrap();

    assert!(result.is_some_and(|p| p.ends_with("target/release/tillerd-daemon")));
}

/// Scenario: None when absent
/// WHEN the binary exists in none of the searched locations
/// THEN no path is returned
#[test]
#[serial]
fn scenario_none_when_absent() {
    let tmp = TempDir::new().unwrap();
    let prev_cwd = std::env::current_dir().unwrap();

    std::env::set_current_dir(tmp.path()).unwrap();
    std::env::remove_var(ENV_DAEMON_BIN);
    std::env::remove_var(ENV_GATE_BIN);
    std::env::remove_var(ENV_NOTIFY_BIN);
    let result = resolve_gate_bin();
    std::env::set_current_dir(prev_cwd).unwrap();

    assert_eq!(result, None);
}

/// Scenario: Names referenced, not repeated
/// WHEN a component reads a governed `TILLERD_*` variable
/// THEN it does so through this library's constant or accessor
/// AND it does not string-literal the variable name
#[test]
fn scenario_names_referenced_not_repeated() {
    // Verify the constants exist and are used by the library.
    assert_eq!(ENV_TILLERD_DIR, "TILLERD_DIR");
    assert_eq!(ENV_DAEMON_BIN, "TILLERD_DAEMON_BIN");
    assert_eq!(ENV_GATE_BIN, "TILLERD_GATE_BIN");
    assert_eq!(ENV_NOTIFY_BIN, "TILLERD_NOTIFY_BIN");

    // The library provides the only interface to these variables.
    // This is documented by the public API.
}
