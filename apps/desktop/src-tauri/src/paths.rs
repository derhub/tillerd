use std::path::PathBuf;

/// `$TILLERD_DIR` or `~/.tillerd` — the shared runtime dir holding the daemon socket and manifest.
pub fn tillerd_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TILLERD_DIR") {
        PathBuf::from(dir)
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".tillerd")
    }
}

pub fn daemon_sock() -> PathBuf {
    tillerd_dir().join("daemon.sock")
}

pub fn manifest_path() -> PathBuf {
    tillerd_dir().join("daemon.json")
}

/// Resolve a service binary by precedence: `$<env_var>`, then `bin/<name>` or
/// `target/{release,debug}/<name>` under the cwd or any ancestor (the cargo
/// build output, so no env is needed in dev/CI), then `~/.local/bin/<name>`.
/// (Packaged-bundle sidecar resolution is §7.)
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
    if let Some(home) = dirs::home_dir() {
        let user_bin = home.join(".local/bin").join(name);
        if user_bin.exists() {
            return Some(user_bin);
        }
    }
    None
}

pub fn resolve_daemon_bin() -> Option<PathBuf> {
    resolve_service_bin("TILLERD_DAEMON_BIN", "tillerd-daemon")
}

pub fn resolve_gate_bin() -> Option<PathBuf> {
    resolve_service_bin("TILLERD_GATE_BIN", "tillerd-gate")
}

/// The committed runtime-free hook callback client (`notify-bash-client`).
pub fn resolve_notify_bin() -> Option<PathBuf> {
    if let Ok(bin) = std::env::var("TILLERD_NOTIFY_BIN") {
        let p = PathBuf::from(bin);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let cwd_bin = cwd.join("bin/tillerd-notify");
        if cwd_bin.exists() {
            return Some(cwd_bin);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn daemon_sock_has_correct_filename() {
        assert_eq!(daemon_sock().file_name().unwrap(), "daemon.sock");
    }

    #[test]
    fn manifest_path_has_correct_filename() {
        assert_eq!(manifest_path().file_name().unwrap(), "daemon.json");
    }

    #[test]
    #[serial]
    fn daemon_sock_and_manifest_share_parent() {
        assert_eq!(daemon_sock().parent(), manifest_path().parent());
    }

    #[test]
    #[serial]
    fn tillerd_dir_uses_env_override() {
        std::env::set_var("TILLERD_DIR", "/tmp/tillerd-test-override");
        let dir = tillerd_dir();
        std::env::remove_var("TILLERD_DIR");
        assert_eq!(dir, std::path::PathBuf::from("/tmp/tillerd-test-override"));
    }

    #[test]
    #[serial]
    fn tillerd_dir_defaults_end_with_dot_tillerd() {
        std::env::remove_var("TILLERD_DIR");
        let dir = tillerd_dir();
        assert_eq!(dir.file_name().unwrap(), ".tillerd");
    }

    #[test]
    #[serial]
    fn resolve_daemon_bin_returns_some_for_existing_env_bin() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("TILLERD_DAEMON_BIN", tmp.path());
        let result = resolve_daemon_bin();
        std::env::remove_var("TILLERD_DAEMON_BIN");
        assert_eq!(result.as_deref(), Some(tmp.path()));
    }

    #[test]
    #[serial]
    fn resolve_daemon_bin_skips_nonexistent_env_bin() {
        let absent = "/nonexistent/path/tillerd-daemon-zzz-test";
        std::env::set_var("TILLERD_DAEMON_BIN", absent);
        let result = resolve_daemon_bin();
        std::env::remove_var("TILLERD_DAEMON_BIN");
        // Must not return the nonexistent path we set.
        assert_ne!(result.as_ref().and_then(|p| p.to_str()), Some(absent));
    }

    #[test]
    #[serial]
    fn resolve_service_bin_discovers_cargo_target_output() {
        let tmp = tempfile::tempdir().unwrap();
        let release = tmp.path().join("target/release");
        std::fs::create_dir_all(&release).unwrap();
        std::fs::write(release.join("tillerd-daemon"), b"x").unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();
        std::env::remove_var("TILLERD_DAEMON_BIN");
        let result = resolve_service_bin("TILLERD_DAEMON_BIN", "tillerd-daemon");
        std::env::set_current_dir(prev).unwrap();

        assert!(result.is_some_and(|p| p.ends_with("target/release/tillerd-daemon")));
    }

    #[test]
    #[serial]
    fn resolve_notify_bin_returns_some_for_existing_env_bin() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("TILLERD_NOTIFY_BIN", tmp.path());
        let result = resolve_notify_bin();
        std::env::remove_var("TILLERD_NOTIFY_BIN");
        assert_eq!(result.as_deref(), Some(tmp.path()));
    }

    #[test]
    #[serial]
    fn resolve_notify_bin_skips_nonexistent_env_bin() {
        let absent = "/nonexistent/path/tillerd-notify-zzz-test";
        std::env::set_var("TILLERD_NOTIFY_BIN", absent);
        let result = resolve_notify_bin();
        std::env::remove_var("TILLERD_NOTIFY_BIN");
        assert_ne!(result.as_ref().and_then(|p| p.to_str()), Some(absent));
    }
}
