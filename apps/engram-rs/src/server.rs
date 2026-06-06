//! Loopback HTTP server: hook ingress (`POST /hook`) + viewer (`GET /`). Blocking
//! and `127.0.0.1`-only; hooks are fire-and-forget.

use crate::Engram;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

/// A routed response: status code, content type, body.
pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl Response {
    fn json(status: u16, body: String) -> Self {
        Self { status, content_type: "application/json", body }
    }
    fn html(body: String) -> Self {
        Self { status: 200, content_type: "text/html; charset=utf-8", body }
    }
}

/// Route a request. Pure over the engram state, so dispatch is unit-testable.
pub fn route(engram: &Engram, method: &str, path: &str, body: &str, now: i64) -> Response {
    match (method, path) {
        ("GET", "/") => Response::html(viewer_html(engram)),
        ("GET", "/healthz") => Response::json(200, r#"{"ok":true}"#.to_string()),
        ("POST", "/hook") => match serde_json::from_str::<Value>(body) {
            Ok(event) => match ingest_hook(engram, &event, now) {
                Ok(id) => Response::json(200, format!(r#"{{"ok":true,"id":{}}}"#, id_json(id))),
                Err(e) => Response::json(500, format!(r#"{{"ok":false,"error":{:?}}}"#, e.to_string())),
            },
            Err(_) => Response::json(400, r#"{"ok":false,"error":"invalid json"}"#.to_string()),
        },
        _ => Response::json(404, r#"{"ok":false,"error":"not found"}"#.to_string()),
    }
}

fn id_json(id: Option<i64>) -> String {
    id.map(|i| i.to_string()).unwrap_or_else(|| "null".to_string())
}

/// Dispatch a hook event to the right capture path. Returns the new chunk id
/// when one was created.
fn ingest_hook(engram: &Engram, event: &Value, now: i64) -> anyhow::Result<Option<i64>> {
    let kind = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let ts = event.get("ts").and_then(|v| v.as_i64()).unwrap_or(now);
    let session_id = event.get("session_id").and_then(|v| v.as_str());

    // A chunk references its session; ensure the row exists for non-start events
    // that arrive before (or without) an explicit session_start.
    if kind != "session_start" {
        if let Some(sid) = session_id {
            let ide = event.get("ide").and_then(|v| v.as_str()).unwrap_or("unknown");
            engram.ensure_session(sid, ide, None, ts)?;
        }
    }

    match kind {
        "session_start" => {
            if let Some(sid) = session_id {
                let ide = event.get("ide").and_then(|v| v.as_str()).unwrap_or("unknown");
                let cwd = event.get("cwd").and_then(|v| v.as_str());
                engram.ensure_session(sid, ide, cwd, ts)?;
            }
            Ok(None)
        }
        "prompt" => {
            let content = event.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let turn = event.get("turn_index").and_then(|v| v.as_i64());
            engram.capture_prompt(session_id, content, turn, ts)
        }
        "tool" => {
            let sid = session_id.unwrap_or("");
            let tool_name = event.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");
            let tool_input = event.get("tool_input").cloned().unwrap_or(Value::Null);
            let tool_response = event.get("tool_response").and_then(|v| v.as_str()).unwrap_or("");
            let turn = event.get("turn_index").and_then(|v| v.as_i64()).unwrap_or(0);
            engram.capture_tool(sid, tool_name, &tool_input, tool_response, turn, ts)
        }
        _ => Ok(None),
    }
}

fn viewer_html(engram: &Engram) -> String {
    let mut rows = String::new();
    if let Ok(chunks) = engram.recent_chunks(100) {
        for (id, title, content) in chunks {
            let t = crate::title_or_prefix(title, &content);
            let snippet: String = content.chars().take(200).collect();
            rows.push_str(&format!(
                "<li><b>#{id}</b> {}<br><small>{}</small></li>",
                escape(&t),
                escape(&snippet)
            ));
        }
    }
    format!(
        "<!doctype html><meta charset=utf-8><title>engram</title>\
         <h1>engram memory</h1><ul>{rows}</ul>"
    )
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Serve on `127.0.0.1:port` until the process exits. Loopback only.
pub fn serve(engram: &Engram, port: u16, now: impl Fn() -> i64) -> anyhow::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    eprintln!("engram viewer on http://127.0.0.1:{port}");
    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).is_err() {
            continue;
        }
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("/").to_string();

        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() || line == "\r\n" || line == "\n" || line.is_empty() {
                break;
            }
            if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = v.trim().parse().unwrap_or(0);
            }
        }
        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            let _ = reader.read_exact(&mut body);
        }
        let body = String::from_utf8_lossy(&body).to_string();

        let resp = route(engram, &method, &path, &body, now());
        let payload = format!(
            "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            resp.status,
            resp.content_type,
            resp.body.len(),
            resp.body
        );
        let _ = stream.write_all(payload.as_bytes());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engram() -> (tempfile::TempDir, Engram) {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("engram.db")).unwrap();
        (dir, e)
    }

    #[test]
    fn hook_prompt_ingests_chunk() {
        let (_d, e) = engram();
        let body = r#"{"type":"prompt","session_id":"s1","content":"hello world","turn_index":0,"ts":5}"#;
        let resp = route(&e, "POST", "/hook", body, 5);
        assert_eq!(resp.status, 200);
        assert_eq!(e.active_chunk_count().unwrap(), 1);
    }

    #[test]
    fn hook_prompt_redacts_secret_before_storage() {
        let (_d, e) = engram();
        let body = r#"{"type":"prompt","session_id":"s1","content":"key ghp_abcdef0123456789abcdefABCDEF0123 ok","turn_index":0,"ts":5}"#;
        route(&e, "POST", "/hook", body, 5);
        let content: String = e
            .store()
            .conn()
            .query_row("SELECT content FROM chunks WHERE id=1", [], |r| r.get(0))
            .unwrap();
        assert!(!content.contains("ghp_"), "secret redacted from stored prompt");
        assert!(content.contains("[REDACTED]"), "marker present");
    }

    #[test]
    fn hook_tool_skips_low_value() {
        let (_d, e) = engram();
        let body = r#"{"type":"tool","session_id":"s1","tool_name":"TodoWrite","tool_response":"x","turn_index":1}"#;
        route(&e, "POST", "/hook", body, 0);
        assert_eq!(e.active_chunk_count().unwrap(), 0, "skip-listed tool not stored");
    }

    #[test]
    fn hook_tool_stores_with_title() {
        let (_d, e) = engram();
        let body = r#"{"type":"tool","session_id":"s1","tool_name":"Read","tool_input":{"file_path":"src/x.rs"},"tool_response":"...","turn_index":2}"#;
        route(&e, "POST", "/hook", body, 0);
        let chunks = e.recent_chunks(10).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].1.as_deref(), Some("Read src/x.rs"));
    }

    #[test]
    fn viewer_renders_html() {
        let (_d, e) = engram();
        route(&e, "POST", "/hook", r#"{"type":"prompt","content":"abc","ts":1}"#, 1);
        let resp = route(&e, "GET", "/", "", 0);
        assert_eq!(resp.content_type, "text/html; charset=utf-8");
        assert!(resp.body.contains("engram memory"));
    }

    #[test]
    fn unknown_route_404() {
        let (_d, e) = engram();
        assert_eq!(route(&e, "GET", "/nope", "", 0).status, 404);
    }

    #[test]
    fn invalid_json_400() {
        let (_d, e) = engram();
        assert_eq!(route(&e, "POST", "/hook", "not json", 0).status, 400);
    }
}
