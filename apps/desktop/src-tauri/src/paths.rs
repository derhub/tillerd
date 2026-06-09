use std::path::PathBuf;

/// `$ATHING_DIR` or `~/.athing` — the shared runtime dir holding the daemon socket and manifest.
pub fn athing_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("ATHING_DIR") {
        PathBuf::from(dir)
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".athing")
    }
}

pub fn daemon_sock() -> PathBuf {
    athing_dir().join("daemon.sock")
}

pub fn manifest_path() -> PathBuf {
    athing_dir().join("daemon.json")
}

/// Resolve the generic PTY daemon binary: `ATHING_DAEMON_BIN`, then a cwd/bundled `bin/athing-daemon`,
/// then `~/.local/bin`. (Packaged-bundle sidecar resolution is §7.)
pub fn resolve_daemon_bin() -> Option<PathBuf> {
    if let Ok(bin) = std::env::var("ATHING_DAEMON_BIN") {
        let p = PathBuf::from(bin);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let cwd_bin = cwd.join("bin/athing-daemon");
        if cwd_bin.exists() {
            return Some(cwd_bin);
        }
    }
    if let Some(home) = dirs::home_dir() {
        let user_bin = home.join(".local/bin/athing-daemon");
        if user_bin.exists() {
            return Some(user_bin);
        }
    }
    None
}

/// The committed runtime-free hook callback client (`notify-bash-client`).
pub fn resolve_notify_bin() -> Option<PathBuf> {
    if let Ok(bin) = std::env::var("ATHING_NOTIFY_BIN") {
        let p = PathBuf::from(bin);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let cwd_bin = cwd.join("bin/athing-notify");
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
    fn daemon_sock_and_manifest_share_parent() {
        assert_eq!(daemon_sock().parent(), manifest_path().parent());
    }

    #[test]
    #[serial]
    fn athing_dir_uses_env_override() {
        std::env::set_var("ATHING_DIR", "/tmp/athing-test-override");
        let dir = athing_dir();
        std::env::remove_var("ATHING_DIR");
        assert_eq!(dir, std::path::PathBuf::from("/tmp/athing-test-override"));
    }

    #[test]
    #[serial]
    fn athing_dir_defaults_end_with_dot_athing() {
        std::env::remove_var("ATHING_DIR");
        let dir = athing_dir();
        assert_eq!(dir.file_name().unwrap(), ".athing");
    }

    #[test]
    #[serial]
    fn resolve_daemon_bin_returns_some_for_existing_env_bin() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("ATHING_DAEMON_BIN", tmp.path());
        let result = resolve_daemon_bin();
        std::env::remove_var("ATHING_DAEMON_BIN");
        assert_eq!(result.as_deref(), Some(tmp.path()));
    }

    #[test]
    #[serial]
    fn resolve_daemon_bin_skips_nonexistent_env_bin() {
        let absent = "/nonexistent/path/athing-daemon-zzz-test";
        std::env::set_var("ATHING_DAEMON_BIN", absent);
        let result = resolve_daemon_bin();
        std::env::remove_var("ATHING_DAEMON_BIN");
        // Must not return the nonexistent path we set.
        assert_ne!(result.as_ref().and_then(|p| p.to_str()), Some(absent));
    }

    #[test]
    #[serial]
    fn resolve_notify_bin_returns_some_for_existing_env_bin() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("ATHING_NOTIFY_BIN", tmp.path());
        let result = resolve_notify_bin();
        std::env::remove_var("ATHING_NOTIFY_BIN");
        assert_eq!(result.as_deref(), Some(tmp.path()));
    }

    #[test]
    #[serial]
    fn resolve_notify_bin_skips_nonexistent_env_bin() {
        let absent = "/nonexistent/path/athing-notify-zzz-test";
        std::env::set_var("ATHING_NOTIFY_BIN", absent);
        let result = resolve_notify_bin();
        std::env::remove_var("ATHING_NOTIFY_BIN");
        assert_ne!(result.as_ref().and_then(|p| p.to_str()), Some(absent));
    }
}
