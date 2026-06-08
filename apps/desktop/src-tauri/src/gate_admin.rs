//! Thin sync client for the gate's admin Unix-socket face.
#![allow(dead_code)]
//!
//! Matches the gate's length-prefixed frame wire exactly: 4-byte big-endian
//! payload length, then JSON. The gate admin request shape is
//! `{adminToken, request: {command, ...}}` (camelCase, tagged by `command`).
//! The gate hashes the token before storing it; the orchestrator sends the raw
//! token and the gate's digest site remains the single point of hashing.

use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use serde_json::{json, Value};

const MAX_FRAME: usize = 1 << 20;

fn write_frame(stream: &mut UnixStream, payload: &[u8]) -> io::Result<()> {
    let len = (payload.len() as u32).to_be_bytes();
    stream.write_all(&len)?;
    stream.write_all(payload)?;
    stream.flush()
}

fn read_frame(stream: &mut UnixStream) -> io::Result<Vec<u8>> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header)?;
    let len = u32::from_be_bytes(header) as usize;
    if len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload)?;
    Ok(payload)
}

fn send_command(sock: &Path, frame: &[u8]) -> io::Result<Value> {
    let mut stream = UnixStream::connect(sock)?;
    write_frame(&mut stream, frame)?;
    let response = read_frame(&mut stream)?;
    serde_json::from_slice(&response).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Register a session with the gate admin face.
///
/// Must be called before spawning the daemon so any hook the daemon agent sends
/// immediately after startup is authenticated (R4/D7).
pub fn register(sock: &Path, admin_token: &str, session_id: &str, token: &str) -> io::Result<()> {
    let frame = serde_json::to_vec(&json!({
        "adminToken": admin_token,
        "request": {
            "command": "register",
            "sessionId": session_id,
            "token": token,
        },
    }))
    .expect("register frame encodes");
    let response = send_command(sock, &frame)?;
    if response.get("result").and_then(Value::as_str) == Some("ok") {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("gate admin register rejected: {response}"),
        ))
    }
}

/// Deregister a session from the gate admin face.
///
/// Called after the daemon PTY session exits so late hooks fail auth.
pub fn deregister(sock: &Path, admin_token: &str, session_id: &str) -> io::Result<()> {
    let frame = serde_json::to_vec(&json!({
        "adminToken": admin_token,
        "request": {
            "command": "deregister",
            "sessionId": session_id,
        },
    }))
    .expect("deregister frame encodes");
    let response = send_command(sock, &frame)?;
    if response.get("result").and_then(Value::as_str) == Some("ok") {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("gate admin deregister rejected: {response}"),
        ))
    }
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

    fn serve_once(sock_path: PathBuf, response: Value) -> mpsc::Receiver<Vec<u8>> {
        let (tx, rx) = mpsc::channel();
        let _ = std::fs::remove_file(&sock_path);
        let listener = UnixListener::bind(&sock_path).unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut header = [0u8; 4];
            stream.read_exact(&mut header).unwrap();
            let len = u32::from_be_bytes(header) as usize;
            let mut payload = vec![0u8; len];
            stream.read_exact(&mut payload).unwrap();
            tx.send(payload).unwrap();
            let resp = serde_json::to_vec(&response).unwrap();
            let resp_header = (resp.len() as u32).to_be_bytes();
            stream.write_all(&resp_header).unwrap();
            stream.write_all(&resp).unwrap();
            stream.flush().unwrap();
        });
        rx
    }

    #[test]
    fn register_sends_camelcase_register_command() {
        let sock = temp_sock("reg");
        let rx = serve_once(sock.clone(), json!({"result": "ok"}));

        register(&sock, "admin-tok", "sess-1", "sess-token").unwrap();

        let frame = rx.recv().unwrap();
        let v: Value = serde_json::from_slice(&frame).unwrap();
        assert_eq!(v["adminToken"], "admin-tok");
        assert_eq!(v["request"]["command"], "register");
        assert_eq!(v["request"]["sessionId"], "sess-1");
        assert_eq!(v["request"]["token"], "sess-token");
        let _ = std::fs::remove_file(&sock);
    }

    #[test]
    fn deregister_sends_camelcase_deregister_command() {
        let sock = temp_sock("dereg");
        let rx = serve_once(sock.clone(), json!({"result": "ok"}));

        deregister(&sock, "admin-tok", "sess-1").unwrap();

        let frame = rx.recv().unwrap();
        let v: Value = serde_json::from_slice(&frame).unwrap();
        assert_eq!(v["adminToken"], "admin-tok");
        assert_eq!(v["request"]["command"], "deregister");
        assert_eq!(v["request"]["sessionId"], "sess-1");
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
