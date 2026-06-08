//! Session-aware daemon orchestrator for the desktop.
#![allow(dead_code)]
//!
//! Uses the process-launch crate directly (no FFI). Before spawning the daemon:
//! - reads ATHING_GATE_URL (or $ATHING_DIR/gate.url) to locate the gate
//! - mints a session id (UUID v4) and a session token (32 random bytes hex)
//! - registers the session with the gate admin face (HARD: must precede spawn)
//!
//! After the daemon PTY session exits the caller deregisters the session so
//! late hooks fail auth. The desktop does NOT install per-project hooks; that
//! is the CLI's responsibility.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use process_launch::{adopt_or_spawn, LaunchError, OsProbes, SpawnTiming};
use uuid::Uuid;

use crate::gate_admin;
use crate::paths::athing_dir;

/// Env-var allowlist for spawn-field diffing (R6).
pub const ENV_ALLOWLIST: &[&str] = &[
    "ATHING_DIR",
    "ATHING_GATE_URL",
    "ATHING_SESSION_ID",
    "ATHING_SESSION_TOKEN",
];

/// A successfully established daemon session: the launched daemon pid plus the
/// minted session credentials injected into the daemon's environment.
pub struct DaemonSession {
    pub pid: u32,
    pub session_id: String,
    pub session_token: String,
    pub athing_dir: PathBuf,
    pub gate_url: Option<String>,
}

/// Mint a random session token: 32 bytes rendered as lowercase hex.
fn mint_token() -> String {
    let bytes: [u8; 32] = std::array::from_fn(|_| rand_byte());
    bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn rand_byte() -> u8 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut h);
    std::thread::current().id().hash(&mut h);
    h.finish() as u8 ^ (h.finish() >> 8) as u8
}

/// Read the gate URL: first from `ATHING_GATE_URL`, then from `$ATHING_DIR/gate.url`.
pub fn resolve_gate_url(base: &Path) -> Option<String> {
    if let Ok(url) = std::env::var("ATHING_GATE_URL") {
        if !url.is_empty() {
            return Some(url);
        }
    }
    let path = base.join("gate.url");
    std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Resolve the gate admin socket path.
pub fn admin_sock(base: &Path) -> PathBuf {
    base.join("gate-admin.sock")
}

/// Adopt-or-spawn the daemon, registering the session with the gate admin face
/// before the daemon starts (R4/D7). Returns a `DaemonSession` on success.
///
/// When no gate admin token is available or the gate admin socket is not
/// reachable, the registration step is skipped and the session credentials are
/// still injected so the daemon can propagate them if the gate appears later.
pub fn ensure_daemon(daemon_bin: &Path, version: &str) -> Result<DaemonSession, String> {
    let base = athing_dir();

    let session_id = Uuid::new_v4().to_string();
    let session_token = mint_token();
    let gate_url = resolve_gate_url(&base);
    let admin_token = std::env::var("ATHING_GATE_ADMIN_TOKEN").unwrap_or_default();

    // Register-before-spawn: gate must know the session before the daemon sends
    // any hook. The desktop does not block on gate absence (soft dependency).
    if gate_url.is_some() && !admin_token.is_empty() {
        let sock = admin_sock(&base);
        if sock.exists() {
            gate_admin::register(&sock, &admin_token, &session_id, &session_token)
                .map_err(|e| format!("gate admin register: {e}"))?;
        }
    }

    let mut env: BTreeMap<String, String> = BTreeMap::new();
    env.insert("ATHING_DIR".into(), base.to_string_lossy().into_owned());
    if let Some(url) = &gate_url {
        env.insert("ATHING_GATE_URL".into(), url.clone());
    }
    env.insert("ATHING_SESSION_ID".into(), session_id.clone());
    env.insert("ATHING_SESSION_TOKEN".into(), session_token.clone());

    let bin = daemon_bin.to_path_buf();
    let env_clone = env.clone();
    let probes = OsProbes::new(move || {
        let mut cmd = Command::new(&bin);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        for (k, v) in &env_clone {
            cmd.env(k, v);
        }
        cmd.spawn()
            .map(|c| c.id())
            .map_err(|e| LaunchError::SpawnFailed(e.to_string()))
    });

    let launched = adopt_or_spawn(&base, version, &SpawnTiming::default(), &probes)
        .map_err(|e| format!("adopt_or_spawn: {e}"))?;

    Ok(DaemonSession {
        pid: launched.pid(),
        session_id,
        session_token,
        athing_dir: base,
        gate_url,
    })
}

/// Deregister the session after the daemon PTY session exits.
///
/// Best-effort: logs but does not propagate errors. Late hooks will fail auth.
pub fn deregister_session(session: &DaemonSession) {
    let admin_token = std::env::var("ATHING_GATE_ADMIN_TOKEN").unwrap_or_default();
    if session.gate_url.is_some() && !admin_token.is_empty() {
        let sock = admin_sock(&session.athing_dir);
        if sock.exists() {
            let _ = gate_admin::deregister(&sock, &admin_token, &session.session_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use serial_test::serial;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::thread;

    fn temp_dir(tag: &str) -> PathBuf {
        // Use /tmp directly; macOS SUN_LEN limit (~104B) requires short socket paths.
        let dir = PathBuf::from(format!("/tmp/ot-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Spawn a fake gate admin socket that accepts one request and records it.
    fn fake_admin_socket(sock_path: &PathBuf) -> mpsc::Receiver<Value> {
        let (tx, rx) = mpsc::channel();
        let _ = std::fs::remove_file(sock_path);
        let listener = UnixListener::bind(sock_path).unwrap();
        let sock_path = sock_path.clone();
        thread::spawn(move || {
            loop {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let tx = tx.clone();
                let sock_path = sock_path.clone();
                thread::spawn(move || {
                    loop {
                        let mut header = [0u8; 4];
                        if stream.read_exact(&mut header).is_err() {
                            break;
                        }
                        let len = u32::from_be_bytes(header) as usize;
                        let mut payload = vec![0u8; len];
                        if stream.read_exact(&mut payload).is_err() {
                            break;
                        }
                        let v: Value = serde_json::from_slice(&payload).unwrap();
                        let _ = tx.send(v);
                        let resp = serde_json::to_vec(&json!({"result": "ok"})).unwrap();
                        let resp_header = (resp.len() as u32).to_be_bytes();
                        let _ = stream.write_all(&resp_header);
                        let _ = stream.write_all(&resp);
                        let _ = stream.flush();
                    }
                    let _ = sock_path; // keep path alive
                });
            }
        });
        rx
    }

    // Verify process-launch types are used directly (not via FFI or wrapper).
    // This test exists purely to assert the import path at the type system level.
    #[test]
    #[expect(
        clippy::type_complexity,
        reason = "type-level assertion that adopt_or_spawn is linked directly from process-launch"
    )]
    fn tauri_uses_process_launch_lib_directly() {
        use process_launch::Launched;
        // OsProbes and adopt_or_spawn are imported from process_launch, confirming
        // the desktop links the crate as a direct Cargo dependency with no FFI.
        let _: fn(
            &Path,
            &str,
            &SpawnTiming,
            &OsProbes<fn() -> Result<u32, LaunchError>>,
        ) -> Result<Launched, LaunchError> = adopt_or_spawn;
    }

    #[test]
    #[serial]
    fn registers_session_before_bootstrap() {
        let dir = temp_dir("reg-before");
        std::env::set_var("ATHING_DIR", &dir);
        std::env::set_var("ATHING_GATE_ADMIN_TOKEN", "test-admin-token");

        // Write a gate URL file so resolve_gate_url finds it.
        std::fs::write(dir.join("gate.url"), "http://127.0.0.1:19999").unwrap();

        let sock_path = admin_sock(&dir);
        let rx = fake_admin_socket(&sock_path);

        // Fake: write a manifest for a live process (ourselves) so adopt succeeds
        // without actually spawning anything.
        let self_pid = std::process::id();
        std::fs::write(
            dir.join("daemon.json"),
            format!(r#"{{"pid":{self_pid},"version":"1.0.0"}}"#),
        )
        .unwrap();
        // The daemon.sock must be reachable; bind a listener on it.
        let daemon_sock = dir.join("daemon.sock");
        let _listener = std::os::unix::net::UnixListener::bind(&daemon_sock).unwrap();

        let fake_bin = PathBuf::from("/bin/sh");
        let session = ensure_daemon(&fake_bin, "1.0.0").unwrap();

        // The register request must have arrived.
        let req = rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert_eq!(req["request"]["command"], "register");
        assert_eq!(req["request"]["sessionId"], session.session_id);
        assert_eq!(req["request"]["token"], session.session_token);
        assert_eq!(req["adminToken"], "test-admin-token");

        std::env::remove_var("ATHING_DIR");
        std::env::remove_var("ATHING_GATE_ADMIN_TOKEN");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn deregisters_session_on_daemon_exit() {
        let dir = temp_dir("dereg-exit");
        std::env::set_var("ATHING_DIR", &dir);
        std::env::set_var("ATHING_GATE_ADMIN_TOKEN", "test-admin-token");
        std::fs::write(dir.join("gate.url"), "http://127.0.0.1:19998").unwrap();

        let sock_path = admin_sock(&dir);
        let rx = fake_admin_socket(&sock_path);

        let session = DaemonSession {
            pid: 0,
            session_id: "test-session-42".to_string(),
            session_token: "test-token-42".to_string(),
            athing_dir: dir.clone(),
            gate_url: Some("http://127.0.0.1:19998".to_string()),
        };

        deregister_session(&session);

        let req = rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert_eq!(req["request"]["command"], "deregister");
        assert_eq!(req["request"]["sessionId"], "test-session-42");
        assert_eq!(req["adminToken"], "test-admin-token");

        std::env::remove_var("ATHING_DIR");
        std::env::remove_var("ATHING_GATE_ADMIN_TOKEN");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
