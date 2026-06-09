//! Daemon orchestrator (desktop). Gate registration HARD: must precede spawn.
//! Deregister on exit so late hooks fail auth. Desktop installs no per-project hooks.
// Not yet invoked from the Tauri command layer; until it is, these entry points are
// reached only by tests, so dead-code analysis would otherwise flag them.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use process_launch::{adopt_or_spawn, LaunchError, OsProbes, SpawnTiming};
use uuid::Uuid;

use crate::gate_admin;
use crate::paths::tillerd_dir;

/// Env-var allowlist for spawn-field diffing (R6).
pub const ENV_ALLOWLIST: &[&str] = &["TILLERD_DIR", "TILLERD_SESSION_ID", "TILLERD_SESSION_TOKEN"];

/// A successfully established daemon session: the launched daemon pid plus the
/// minted session credentials injected into the daemon's environment.
pub struct DaemonSession {
    pub pid: u32,
    pub session_id: String,
    pub session_token: String,
    pub tillerd_dir: PathBuf,
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

/// Resolve the gate's single socket path; the orchestrator reaches the admin face
/// over its `Admin` route. Its presence is the signal the gate is up.
pub fn admin_sock(base: &Path) -> PathBuf {
    base.join("gate.sock")
}

/// Adopt-or-spawn the daemon, registering the session with the gate admin face
/// before the daemon starts (R4/D7). Returns a `DaemonSession` on success.
///
/// When no gate admin token is available or the gate admin socket is not
/// reachable, the registration step is skipped and the session credentials are
/// still injected so the daemon can propagate them if the gate appears later.
pub fn ensure_daemon(daemon_bin: &Path, version: &str) -> Result<DaemonSession, String> {
    let base = tillerd_dir();

    let session_id = Uuid::new_v4().to_string();
    let session_token = mint_token();
    let admin_token = std::env::var("TILLERD_GATE_ADMIN_TOKEN").unwrap_or_default();

    // Register-before-spawn: gate must know the session before the daemon sends
    // any hook. The desktop does not block on gate absence (soft dependency): the
    // gate admin socket's presence is the signal the gate is up.
    if !admin_token.is_empty() {
        let sock = admin_sock(&base);
        if sock.exists() {
            gate_admin::register(&sock, &admin_token, &session_id, &session_token)
                .map_err(|e| format!("gate admin register: {e}"))?;
        }
    }

    let mut env: BTreeMap<String, String> = BTreeMap::new();
    env.insert("TILLERD_DIR".into(), base.to_string_lossy().into_owned());
    env.insert("TILLERD_SESSION_ID".into(), session_id.clone());
    env.insert("TILLERD_SESSION_TOKEN".into(), session_token.clone());

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
        tillerd_dir: base,
    })
}

/// Deregister the session after the daemon PTY session exits.
///
/// Best-effort: logs but does not propagate errors. Late hooks will fail auth.
pub fn deregister_session(session: &DaemonSession) {
    let admin_token = std::env::var("TILLERD_GATE_ADMIN_TOKEN").unwrap_or_default();
    if !admin_token.is_empty() {
        let sock = admin_sock(&session.tillerd_dir);
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

    /// Spawn a fake gate that accepts the `Admin` route: per connection it reads the
    /// preamble then the command frame, records both, and replies `ok` once.
    fn fake_admin_socket(sock_path: &PathBuf) -> mpsc::Receiver<(Value, Value)> {
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
                    let read_frame =
                        |stream: &mut std::os::unix::net::UnixStream| -> Option<Value> {
                            let mut header = [0u8; 4];
                            stream.read_exact(&mut header).ok()?;
                            let len = u32::from_be_bytes(header) as usize;
                            let mut payload = vec![0u8; len];
                            stream.read_exact(&mut payload).ok()?;
                            serde_json::from_slice(&payload).ok()
                        };
                    let (Some(preamble), Some(command)) =
                        (read_frame(&mut stream), read_frame(&mut stream))
                    else {
                        return;
                    };
                    let _ = tx.send((preamble, command));
                    let resp = serde_json::to_vec(&json!({"result": "ok"})).unwrap();
                    let resp_header = (resp.len() as u32).to_be_bytes();
                    let _ = stream.write_all(&resp_header);
                    let _ = stream.write_all(&resp);
                    let _ = stream.flush();
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
        std::env::set_var("TILLERD_DIR", &dir);
        std::env::set_var("TILLERD_GATE_ADMIN_TOKEN", "test-admin-token");

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

        // The register request must have arrived: an admin preamble then the command.
        let (preamble, command) = rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert_eq!(preamble["route"], "admin");
        assert_eq!(preamble["token"], "test-admin-token");
        assert_eq!(command["command"], "register");
        assert_eq!(command["sessionId"], session.session_id);
        assert_eq!(command["token"], session.session_token);

        std::env::remove_var("TILLERD_DIR");
        std::env::remove_var("TILLERD_GATE_ADMIN_TOKEN");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn deregisters_session_on_daemon_exit() {
        let dir = temp_dir("dereg-exit");
        std::env::set_var("TILLERD_DIR", &dir);
        std::env::set_var("TILLERD_GATE_ADMIN_TOKEN", "test-admin-token");

        let sock_path = admin_sock(&dir);
        let rx = fake_admin_socket(&sock_path);

        let session = DaemonSession {
            pid: 0,
            session_id: "test-session-42".to_string(),
            session_token: "test-token-42".to_string(),
            tillerd_dir: dir.clone(),
        };

        deregister_session(&session);

        let (preamble, command) = rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert_eq!(preamble["route"], "admin");
        assert_eq!(preamble["token"], "test-admin-token");
        assert_eq!(command["command"], "deregister");
        assert_eq!(command["sessionId"], "test-session-42");

        std::env::remove_var("TILLERD_DIR");
        std::env::remove_var("TILLERD_GATE_ADMIN_TOKEN");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
