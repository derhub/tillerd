//! Optional loopback hook receiver: minimal HTTP POST endpoint over a Unix
//! socket, per-session token auth (constant-time), idempotency dedup, relaying
//! authenticated raw payloads to subscribers.

use crate::server::Daemon;
use std::collections::HashSet;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

pub struct HookIngress {
    task: tokio::task::JoinHandle<()>,
    socket_path: PathBuf,
}

impl HookIngress {
    pub fn start(socket_path: PathBuf, daemon: Daemon) -> HookIngress {
        let _ = std::fs::remove_file(&socket_path);
        let listener = match UnixListener::bind(&socket_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("hook-ingress: bind failed: {e}");
                // Return an inert handle.
                return HookIngress {
                    task: tokio::spawn(async {}),
                    socket_path,
                };
            }
        };
        if let Ok(meta) = std::fs::metadata(&socket_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            let _ = std::fs::set_permissions(&socket_path, perms);
        }

        let dedup = Arc::new(Mutex::new(HashSet::<String>::new()));
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let daemon = daemon.clone();
                let dedup = dedup.clone();
                tokio::spawn(async move {
                    let _ = handle_request(stream, daemon, dedup).await;
                });
            }
        });

        HookIngress { task, socket_path }
    }

    pub fn stop(&self) {
        self.task.abort();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

impl Drop for HookIngress {
    fn drop(&mut self) {
        self.stop();
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

async fn respond(stream: &mut UnixStream, status: &str, body: &str) {
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.flush().await;
}

async fn handle_request(
    mut stream: UnixStream,
    daemon: Daemon,
    dedup: Arc<Mutex<HashSet<String>>>,
) -> std::io::Result<()> {
    let mut data = Vec::new();
    let mut buf = [0u8; 8192];
    let header_end;
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            respond(&mut stream, "400 Bad Request", "bad request").await;
            return Ok(());
        }
        data.extend_from_slice(&buf[..n]);
        if let Some(pos) = find_subslice(&data, b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
        if data.len() > 1_048_576 {
            respond(&mut stream, "400 Bad Request", "bad request").await;
            return Ok(());
        }
    }

    let head = String::from_utf8_lossy(&data[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let method = request_line.split(' ').next().unwrap_or("");
    if method != "POST" {
        respond(&mut stream, "405 Method Not Allowed", "method not allowed").await;
        return Ok(());
    }

    let mut session_id = String::new();
    let mut token = String::new();
    let mut content_length = 0usize;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim();
            match key.as_str() {
                "x-session-id" => session_id = val.to_string(),
                "x-session-token" => token = val.to_string(),
                "content-length" => content_length = val.parse().unwrap_or(0),
                _ => {}
            }
        }
    }

    if session_id.is_empty() || token.is_empty() {
        respond(&mut stream, "401 Unauthorized", "unauthorized").await;
        return Ok(());
    }

    let mut body = data[header_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);
    }
    body.truncate(content_length);

    let Some(expected) = daemon.session_token(&session_id) else {
        respond(&mut stream, "403 Forbidden", "forbidden").await;
        return Ok(());
    };
    if !constant_time_eq(token.as_bytes(), expected.as_bytes()) {
        respond(&mut stream, "403 Forbidden", "forbidden").await;
        return Ok(());
    }

    let payload: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            respond(&mut stream, "400 Bad Request", "bad request").await;
            return Ok(());
        }
    };

    let key = format!(
        "{session_id}:{}",
        serde_json::to_string(&payload).unwrap_or_default()
    );
    let is_dup = {
        let mut set = dedup.lock().unwrap();
        if set.contains(&key) {
            true
        } else {
            set.insert(key);
            if set.len() > 10_000 {
                let to_drop: Vec<String> = set.iter().take(1_000).cloned().collect();
                for k in to_drop {
                    set.remove(&k);
                }
            }
            false
        }
    };
    if is_dup {
        respond(&mut stream, "200 OK", "ok").await;
        return Ok(());
    }

    daemon.relay_hook(&session_id, payload);
    respond(&mut stream, "200 OK", "ok").await;
    Ok(())
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_basic() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }

    #[test]
    fn find_header_terminator() {
        assert_eq!(
            find_subslice(b"GET / HTTP\r\n\r\nbody", b"\r\n\r\n"),
            Some(10)
        );
    }
}
