#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Single source of truth for the tillerd runtime layout and the `TILLERD_*`
//! environment surface.
//!
//! Resolves the runtime directory (`TILLERD_DIR` → `~/.tillerd`), builds the
//! socket/manifest/store paths under it, discovers the service binaries by a
//! defined precedence, and is the one place the governed `TILLERD_*` variable
//! names are defined. Every service and the host depend on it; no other crate
//! resolves these paths or reads these variables directly.

use std::path::{Path, PathBuf};

/// Name of the environment variable that overrides the runtime directory.
pub const ENV_TILLERD_DIR: &str = "TILLERD_DIR";
/// Name of the environment variable that overrides the daemon binary path.
pub const ENV_DAEMON_BIN: &str = "TILLERD_DAEMON_BIN";
/// Name of the environment variable that overrides the gate binary path.
pub const ENV_GATE_BIN: &str = "TILLERD_GATE_BIN";
/// Name of the environment variable that overrides the notify binary path.
pub const ENV_NOTIFY_BIN: &str = "TILLERD_NOTIFY_BIN";

/// Resolve the runtime directory from the environment: `TILLERD_DIR` when set
/// (used exactly), otherwise `~/.tillerd`.
pub fn runtime_dir() -> PathBuf {
    runtime_dir_or(None)
}

/// Resolve the runtime directory honoring an explicit override first, then
/// `TILLERD_DIR`, then `~/.tillerd`. A set value is used exactly as given.
pub fn runtime_dir_or(base_override: Option<&str>) -> PathBuf {
    base_override
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            std::env::var(ENV_TILLERD_DIR)
                .ok()
                .filter(|v| !v.is_empty())
        })
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".tillerd"))
}

/// The daemon control socket under `dir`.
pub fn daemon_socket_in(dir: &Path) -> PathBuf {
    dir.join("daemon.sock")
}

/// The gate front-door socket under `dir`.
pub fn gate_socket_in(dir: &Path) -> PathBuf {
    dir.join("gate.sock")
}

/// The daemon manifest under `dir`.
pub fn manifest_in(dir: &Path) -> PathBuf {
    dir.join("daemon.json")
}

/// The product store under `dir`.
pub fn store_in(dir: &Path) -> PathBuf {
    dir.join("tillerd.db")
}

/// The daemon control socket under the resolved runtime directory.
pub fn daemon_socket() -> PathBuf {
    daemon_socket_in(&runtime_dir())
}

/// The gate front-door socket under the resolved runtime directory.
pub fn gate_socket() -> PathBuf {
    gate_socket_in(&runtime_dir())
}

/// The daemon manifest under the resolved runtime directory.
pub fn manifest() -> PathBuf {
    manifest_in(&runtime_dir())
}

/// The product store under the resolved runtime directory.
pub fn store() -> PathBuf {
    store_in(&runtime_dir())
}

/// Resolve the daemon service binary by the shared precedence.
pub fn resolve_daemon_bin() -> Option<PathBuf> {
    resolve_service_bin(ENV_DAEMON_BIN, "tillerd-daemon")
}

/// Resolve the gate service binary by the shared precedence.
pub fn resolve_gate_bin() -> Option<PathBuf> {
    resolve_service_bin(ENV_GATE_BIN, "tillerd-gate")
}

/// Resolve the notify hook-callback binary by the shared precedence.
pub fn resolve_notify_bin() -> Option<PathBuf> {
    resolve_service_bin(ENV_NOTIFY_BIN, "tillerd-notify")
}

/// Resolve a service binary by precedence: `$<env_var>` when it names an
/// existing file, then `bin/<name>` or the cargo build output
/// `target/{release,debug}/<name>` under the working directory or any ancestor,
/// then `~/.local/bin/<name>`. Returns `None` when none exists.
fn resolve_service_bin(env_var: &str, name: &str) -> Option<PathBuf> {
    if let Ok(bin) = std::env::var(env_var) {
        let p = PathBuf::from(bin);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for dir in cwd.ancestors() {
            for candidate in [
                dir.join("bin").join(name),
                dir.join("target/release").join(name),
                dir.join("target/debug").join(name),
            ] {
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    let user_bin = home_dir().join(".local/bin").join(name);
    if user_bin.exists() {
        return Some(user_bin);
    }
    None
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn runtime_dir_uses_the_env_override_exactly() {
        std::env::set_var(ENV_TILLERD_DIR, "/tmp/tillerd-paths-override");
        let dir = runtime_dir();
        std::env::remove_var(ENV_TILLERD_DIR);
        assert_eq!(dir, PathBuf::from("/tmp/tillerd-paths-override"));
    }

    #[test]
    #[serial]
    fn runtime_dir_defaults_to_dot_tillerd_under_home() {
        std::env::remove_var(ENV_TILLERD_DIR);
        let dir = runtime_dir();
        assert_eq!(dir.file_name().unwrap(), ".tillerd");
    }

    #[test]
    #[serial]
    fn runtime_dir_or_prefers_the_explicit_override_over_the_env() {
        std::env::set_var(ENV_TILLERD_DIR, "/tmp/from-env");
        let dir = runtime_dir_or(Some("/tmp/from-arg"));
        std::env::remove_var(ENV_TILLERD_DIR);
        assert_eq!(dir, PathBuf::from("/tmp/from-arg"));
    }

    #[test]
    #[serial]
    fn runtime_dir_or_falls_back_to_the_env_when_no_override() {
        std::env::set_var(ENV_TILLERD_DIR, "/tmp/from-env-only");
        let dir = runtime_dir_or(None);
        std::env::remove_var(ENV_TILLERD_DIR);
        assert_eq!(dir, PathBuf::from("/tmp/from-env-only"));
    }

    #[test]
    #[serial]
    fn runtime_dir_or_falls_back_to_the_default_when_unset() {
        std::env::remove_var(ENV_TILLERD_DIR);
        let dir = runtime_dir_or(None);
        assert_eq!(dir.file_name().unwrap(), ".tillerd");
    }

    #[test]
    fn socket_manifest_and_store_paths_are_deterministic_under_a_dir() {
        let dir = Path::new("/base");
        assert_eq!(daemon_socket_in(dir), PathBuf::from("/base/daemon.sock"));
        assert_eq!(gate_socket_in(dir), PathBuf::from("/base/gate.sock"));
        assert_eq!(manifest_in(dir), PathBuf::from("/base/daemon.json"));
        assert_eq!(store_in(dir), PathBuf::from("/base/tillerd.db"));
    }

    #[test]
    fn all_layout_paths_share_the_runtime_dir_as_parent() {
        let dir = Path::new("/srv/tillerd");
        assert_eq!(daemon_socket_in(dir).parent(), Some(dir));
        assert_eq!(gate_socket_in(dir).parent(), Some(dir));
        assert_eq!(manifest_in(dir).parent(), Some(dir));
        assert_eq!(store_in(dir).parent(), Some(dir));
    }

    #[test]
    #[serial]
    fn binary_resolution_returns_the_override_when_it_exists() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var(ENV_DAEMON_BIN, tmp.path());
        let result = resolve_daemon_bin();
        std::env::remove_var(ENV_DAEMON_BIN);
        assert_eq!(result.as_deref(), Some(tmp.path()));
    }

    #[test]
    #[serial]
    fn binary_resolution_skips_the_override_when_it_is_missing() {
        let cwd = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(cwd.path()).unwrap();
        std::env::set_var(ENV_DAEMON_BIN, "/nonexistent/tillerd-daemon-zzz");
        let result = resolve_service_bin(ENV_DAEMON_BIN, "tillerd-daemon-zzz-absent");
        std::env::remove_var(ENV_DAEMON_BIN);
        std::env::set_current_dir(prev).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[serial]
    fn binary_resolution_discovers_the_cargo_target_output_without_env() {
        let tmp = tempfile::tempdir().unwrap();
        let release = tmp.path().join("target/release");
        std::fs::create_dir_all(&release).unwrap();
        std::fs::write(release.join("tillerd-daemon"), b"x").unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();
        std::env::remove_var(ENV_DAEMON_BIN);
        let result = resolve_service_bin(ENV_DAEMON_BIN, "tillerd-daemon");
        std::env::set_current_dir(prev).unwrap();

        assert!(result.is_some_and(|p| p.ends_with("target/release/tillerd-daemon")));
    }

    #[test]
    #[serial]
    fn binary_resolution_returns_none_when_absent_everywhere() {
        let cwd = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(cwd.path()).unwrap();
        std::env::remove_var(ENV_GATE_BIN);
        let result = resolve_service_bin(ENV_GATE_BIN, "tillerd-absent-binary-zzz");
        std::env::set_current_dir(prev).unwrap();
        assert_eq!(result, None);
    }
}
