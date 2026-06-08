//! Unauthenticated liveness probe.
//!
//! A tool started through the host exposes a cheap, credential-free reachability
//! check over a loopback Unix socket: `GET /health` returns the tool's version
//! and reachability so a launcher can detect a running instance (and its
//! version) before holding any credential. Anything that is not the health
//! request is rejected, but no request is ever authenticated.

use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

/// The unauthenticated liveness path.
pub const HEALTH_PATH: &str = "/health";

/// Build the body a successful probe returns: reachability + version JSON.
pub fn liveness_body(version: &str) -> String {
    format!(r#"{{"reachable":true,"version":"{version}"}}"#)
}

/// A running liveness probe bound to a loopback Unix socket.
pub struct Probe {
    task: tokio::task::JoinHandle<()>,
    socket_path: PathBuf,
}

impl Probe {
    /// Bind the probe to `socket_path`, serving `version` to liveness requests.
    pub fn start(socket_path: PathBuf, version: String) -> std::io::Result<Probe> {
        // A stale socket from a dead instance must be cleared before binding.
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path)?;

        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let version = version.clone();
                tokio::spawn(async move {
                    handle(stream, &version).await;
                });
            }
        });

        Ok(Probe { task, socket_path })
    }

    /// Stop serving the probe and remove its socket.
    pub fn stop(&self) {
        self.task.abort();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn handle(mut stream: UnixStream, version: &str) {
    let mut buf = vec![0u8; 1024];
    let n = match stream.read(&mut buf).await {
        Ok(n) => n,
        Err(_) => return,
    };
    let request = String::from_utf8_lossy(&buf[..n]);
    let first_line = request.lines().next().unwrap_or_default();

    if is_health_request(first_line) {
        let body = liveness_body(version);
        respond(&mut stream, "200 OK", &body).await;
    } else {
        respond(&mut stream, "400 Bad Request", "").await;
    }
}

fn is_health_request(request_line: &str) -> bool {
    let mut parts = request_line.split_whitespace();
    matches!(
        (parts.next(), parts.next()),
        (Some("GET"), Some(HEALTH_PATH))
    )
}

async fn respond(stream: &mut UnixStream, status: &str, body: &str) {
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.flush().await;
}

/// Send a raw liveness request to a probe socket and return the parsed
/// `(status, body)`. A small client used by callers and tests.
pub async fn probe_once(socket_path: &Path, request: &str) -> std::io::Result<(String, String)> {
    let mut stream = UnixStream::connect(socket_path).await?;
    stream.write_all(request.as_bytes()).await?;
    stream.flush().await?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await?;
    let text = String::from_utf8_lossy(&raw).to_string();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("HTTP/1.1 "))
        .unwrap_or_default()
        .to_string();
    Ok((status, body.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unix socket paths are capped (~104 bytes on macOS), so keep them short
    // and under /tmp rather than the long system temp dir.
    fn temp_sock(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        PathBuf::from(format!(
            "/tmp/sh-p-{tag}-{}-{nanos}.sock",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn liveness_endpoint_responds_without_auth() {
        let sock = temp_sock("no-auth");
        let probe = Probe::start(sock.clone(), "1.0.0".into()).unwrap();

        let (status, _body) = probe_once(&sock, "GET /health HTTP/1.1\r\n\r\n")
            .await
            .unwrap();
        assert_eq!(status, "200 OK", "no credential is required");
        probe.stop();
    }

    #[tokio::test]
    async fn liveness_returns_version_and_reachability() {
        let sock = temp_sock("version");
        let probe = Probe::start(sock.clone(), "4.5.6".into()).unwrap();

        let (_status, body) = probe_once(&sock, "GET /health HTTP/1.1\r\n\r\n")
            .await
            .unwrap();
        assert!(body.contains("\"reachable\":true"), "reports reachability");
        assert!(body.contains("\"version\":\"4.5.6\""), "reports version");
        probe.stop();
    }

    #[tokio::test]
    async fn liveness_rejects_invalid_requests() {
        let sock = temp_sock("invalid");
        let probe = Probe::start(sock.clone(), "1.0.0".into()).unwrap();

        let (status, _) = probe_once(&sock, "POST /health HTTP/1.1\r\n\r\n")
            .await
            .unwrap();
        assert_eq!(status, "400 Bad Request", "non-GET is rejected");

        let (status, _) = probe_once(&sock, "GET /other HTTP/1.1\r\n\r\n")
            .await
            .unwrap();
        assert_eq!(status, "400 Bad Request", "wrong path is rejected");
        probe.stop();
    }
}
