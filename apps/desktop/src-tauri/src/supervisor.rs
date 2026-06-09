use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::State;
use tokio::net::UnixStream;

use crate::paths::{daemon_sock, manifest_path, resolve_daemon_bin};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    /// We spawned the daemon; we terminate it on exit.
    Owned,
    /// A pre-existing daemon we connected to; we leave it running on exit (ADR-0007).
    Adopted,
}

#[derive(Default)]
pub struct SupervisorState {
    /// `Some((pid, ownership))` once the daemon is ensured.
    inner: Mutex<Option<(u32, Ownership)>>,
}

#[derive(Serialize)]
pub struct EnsureResult {
    pub ownership: &'static str,
    pub socket: String,
}

fn read_manifest() -> Option<(u32, String)> {
    let raw = std::fs::read(manifest_path()).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&raw).ok()?;
    let pid = v.get("pid")?.as_u64()? as u32;
    let version = v.get("version")?.as_str()?.to_string();
    Some((pid, version))
}

fn is_alive(pid: u32) -> bool {
    // SAFETY: signal 0 probes process existence without delivering a signal.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

async fn socket_reachable() -> bool {
    UnixStream::connect(daemon_sock()).await.is_ok()
}

/// Adopt a live daemon recorded in the manifest, else spawn one and wait for reachability.
#[tauri::command]
pub async fn daemon_ensure(state: State<'_, SupervisorState>) -> Result<EnsureResult, String> {
    if let Some((pid, _version)) = read_manifest() {
        if is_alive(pid) && socket_reachable().await {
            *state.inner.lock().unwrap() = Some((pid, Ownership::Adopted));
            return Ok(EnsureResult {
                ownership: "adopted",
                socket: sock_string(),
            });
        }
    }

    // Stale socket from a dead daemon blocks bind; remove best-effort.
    let _ = std::fs::remove_file(daemon_sock());

    let bin = resolve_daemon_bin()
        .ok_or_else(|| "cannot resolve tillerd-daemon binary (set TILLERD_DAEMON_BIN)".to_string())?;
    let child = Command::new(&bin)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn {}: {}", bin.display(), e))?;
    let pid = child.id();

    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if socket_reachable().await {
            *state.inner.lock().unwrap() = Some((pid, Ownership::Owned));
            return Ok(EnsureResult {
                ownership: "owned",
                socket: sock_string(),
            });
        }
    }
    Err("daemon did not become reachable within 10s".into())
}

fn sock_string() -> String {
    daemon_sock().to_string_lossy().into_owned()
}

/// On app exit: SIGTERM an owned daemon (graceful), leave an adopted daemon running.
pub fn shutdown_owned(state: &SupervisorState) {
    if let Some((pid, Ownership::Owned)) = *state.inner.lock().unwrap() {
        unsafe {
            libc::kill(pid as libc::pid_t, libc::SIGTERM);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn read_manifest_returns_none_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("TILLERD_DIR", tmp.path());
        let result = read_manifest();
        std::env::remove_var("TILLERD_DIR");
        assert!(result.is_none());
    }

    #[test]
    #[serial]
    fn read_manifest_parses_valid_json() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("daemon.json"),
            br#"{"pid":12345,"version":"1.2.3"}"#,
        )
        .unwrap();
        std::env::set_var("TILLERD_DIR", tmp.path());
        let result = read_manifest();
        std::env::remove_var("TILLERD_DIR");
        assert_eq!(result, Some((12345, "1.2.3".to_string())));
    }

    #[test]
    #[serial]
    fn read_manifest_returns_none_for_invalid_json() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("daemon.json"), b"not json").unwrap();
        std::env::set_var("TILLERD_DIR", tmp.path());
        let result = read_manifest();
        std::env::remove_var("TILLERD_DIR");
        assert!(result.is_none());
    }

    #[test]
    fn shutdown_owned_is_noop_when_no_daemon() {
        let state = SupervisorState::default();
        shutdown_owned(&state);
    }

    #[test]
    fn shutdown_owned_is_noop_for_adopted_daemon() {
        let state = SupervisorState::default();
        *state.inner.lock().unwrap() = Some((99999, Ownership::Adopted));
        shutdown_owned(&state); // must not SIGTERM an adopted daemon
    }

    #[test]
    fn is_alive_returns_true_for_current_process() {
        let pid = std::process::id();
        assert!(is_alive(pid));
    }

    #[test]
    fn is_alive_returns_false_for_nonexistent_pid() {
        // PID 0 is not a valid target for kill(2) from userspace on macOS.
        // Use a very large PID that is virtually guaranteed not to exist.
        assert!(!is_alive(99999999));
    }
}
