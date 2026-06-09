//! Thin sync client for the gate's `Admin` route on its single Unix socket.
#![allow(dead_code)]
//!
//! Matches the gate's length-prefixed frame wire exactly: 4-byte big-endian
//! payload length, then JSON. Each call opens the gate socket on the `Admin` route:
//! one preamble frame `{route:"admin", token, wireVersion}` carrying the admin
//! token, then one bare command frame `{command, ...}` (camelCase, tagged by
//! `command`). The gate hashes the token before storing it; the orchestrator sends
//! the raw token and the gate's digest site remains the single point of hashing.

use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use contracts::framing::{encode_frame, MAX_FRAME_SIZE};
use contracts::HOOK_SUBSCRIPTION_WIRE_VERSION;
use serde_json::{json, Value};

fn write_frame(stream: &mut UnixStream, payload: &[u8]) -> io::Result<()> {
    stream.write_all(&encode_frame(payload))?;
    stream.flush()
}

fn read_frame(stream: &mut UnixStream) -> io::Result<Vec<u8>> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header)?;
    let len = u32::from_be_bytes(header) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload)?;
    Ok(payload)
}

/// Open the gate socket on the `Admin` route and send one command: the preamble
/// carries the admin token, then the bare command frame; returns the gate's reply.
fn send_command(sock: &Path, admin_token: &str, command: &[u8]) -> io::Result<Value> {
    let preamble = serde_json::to_vec(&json!({
        "route": "admin",
        "token": admin_token,
        "wireVersion": HOOK_SUBSCRIPTION_WIRE_VERSION,
    }))
    .expect("admin preamble encodes");
    let mut stream = UnixStream::connect(sock)?;
    write_frame(&mut stream, &preamble)?;
    write_frame(&mut stream, command)?;
    let response = read_frame(&mut stream)?;
    serde_json::from_slice(&response).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn check_ok(response: Value, what: &str) -> io::Result<()> {
    if response.get("result").and_then(Value::as_str) == Some("ok") {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("gate admin {what} rejected: {response}"),
        ))
    }
}

/// Register a session with the gate's `Admin` route.
///
/// Must be called before spawning the daemon so any hook the daemon agent sends
/// immediately after startup is authenticated (R4/D7).
pub fn register(sock: &Path, admin_token: &str, session_id: &str, token: &str) -> io::Result<()> {
    let command = serde_json::to_vec(&json!({
        "command": "register",
        "sessionId": session_id,
        "token": token,
    }))
    .expect("register command encodes");
    check_ok(send_command(sock, admin_token, &command)?, "register")
}

/// Deregister a session from the gate's `Admin` route.
///
/// Called after the daemon PTY session exits so late hooks fail auth.
pub fn deregister(sock: &Path, admin_token: &str, session_id: &str) -> io::Result<()> {
    let command = serde_json::to_vec(&json!({
        "command": "deregister",
        "sessionId": session_id,
    }))
    .expect("deregister command encodes");
    check_ok(send_command(sock, admin_token, &command)?, "deregister")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::thread;

    fn temp_sock(tag: &str) -> PathBuf {
        // Use /tmp directly; macOS SUN_LEN limit (~104B) requires short paths.
        PathBuf::from(format!("/tmp/ga-{tag}-{}.sock", std::process::id(),))
    }

    fn read_one_frame(stream: &mut std::os::unix::net::UnixStream) -> Vec<u8> {
        let mut header = [0u8; 4];
        stream.read_exact(&mut header).unwrap();
        let len = u32::from_be_bytes(header) as usize;
        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload).unwrap();
        payload
    }

    /// A fake gate `Admin` route: reads the preamble then the command frame, sends
    /// both back for assertions, and replies once with `response`.
    fn serve_once(sock_path: PathBuf, response: Value) -> mpsc::Receiver<(Value, Value)> {
        let (tx, rx) = mpsc::channel();
        let _ = std::fs::remove_file(&sock_path);
        let listener = UnixListener::bind(&sock_path).unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let preamble: Value = serde_json::from_slice(&read_one_frame(&mut stream)).unwrap();
            let command: Value = serde_json::from_slice(&read_one_frame(&mut stream)).unwrap();
            tx.send((preamble, command)).unwrap();
            let resp = serde_json::to_vec(&response).unwrap();
            let resp_header = (resp.len() as u32).to_be_bytes();
            stream.write_all(&resp_header).unwrap();
            stream.write_all(&resp).unwrap();
            stream.flush().unwrap();
        });
        rx
    }

    #[test]
    fn register_sends_an_admin_preamble_then_a_register_command() {
        let sock = temp_sock("reg");
        let rx = serve_once(sock.clone(), json!({"result": "ok"}));

        register(&sock, "admin-tok", "sess-1", "sess-token").unwrap();

        let (preamble, command) = rx.recv().unwrap();
        assert_eq!(preamble["route"], "admin");
        assert_eq!(preamble["token"], "admin-tok");
        assert_eq!(command["command"], "register");
        assert_eq!(command["sessionId"], "sess-1");
        assert_eq!(command["token"], "sess-token");
        let _ = std::fs::remove_file(&sock);
    }

    #[test]
    fn deregister_sends_an_admin_preamble_then_a_deregister_command() {
        let sock = temp_sock("dereg");
        let rx = serve_once(sock.clone(), json!({"result": "ok"}));

        deregister(&sock, "admin-tok", "sess-1").unwrap();

        let (preamble, command) = rx.recv().unwrap();
        assert_eq!(preamble["route"], "admin");
        assert_eq!(preamble["token"], "admin-tok");
        assert_eq!(command["command"], "deregister");
        assert_eq!(command["sessionId"], "sess-1");
        let _ = std::fs::remove_file(&sock);
    }

    #[test]
    fn register_returns_error_when_gate_rejects() {
        let sock = temp_sock("reg-reject");
        let _rx = serve_once(sock.clone(), json!({"result": "unauthenticated"}));

        let err = register(&sock, "wrong", "s", "t").unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        let _ = std::fs::remove_file(&sock);
    }
}
