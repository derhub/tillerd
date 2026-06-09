//! Server: loopback HTTP viewer + health. Blocking, localhost-only.

use crate::Engram;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

/// A routed response: status code, content type, body.
pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl Response {
    fn json(status: u16, body: String) -> Self {
        Self {
            status,
            content_type: "application/json",
            body,
        }
    }
    fn html(body: String) -> Self {
        Self {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body,
        }
    }
}

/// Route a request. Pure over the memorya state, so dispatch is unit-testable.
pub fn route(memorya: &Engram, method: &str, path: &str) -> Response {
    match (method, path) {
        ("GET", "/") => Response::html(viewer_html(memorya)),
        ("GET", "/healthz") => Response::json(200, r#"{"ok":true}"#.to_string()),
        _ => Response::json(404, r#"{"ok":false,"error":"not found"}"#.to_string()),
    }
}

fn viewer_html(memorya: &Engram) -> String {
    let mut rows = String::new();
    if let Ok(chunks) = memorya.recent_chunks(100) {
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
        "<!doctype html><meta charset=utf-8><title>memorya</title>\
         <h1>memorya memory</h1><ul>{rows}</ul>"
    )
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Bind the viewer's loopback listener. `127.0.0.1` only, in every mode.
pub fn bind(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind(("127.0.0.1", port))
}

/// Serve the viewer until the process exits. Each request locks the shared
/// memorya only for the duration of its dispatch, so capture and the embedding
/// worker keep running between requests.
pub fn serve(memorya: Arc<Mutex<Engram>>, port: u16) -> anyhow::Result<()> {
    let listener = bind(port)?;
    eprintln!("memorya viewer on http://127.0.0.1:{port}");
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

        // Drain the request headers; the viewer's routes carry no request body.
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_err()
                || line == "\r\n"
                || line == "\n"
                || line.is_empty()
            {
                break;
            }
        }

        let resp = {
            let memorya = memorya.lock().expect("memorya mutex poisoned");
            route(&memorya, &method, &path)
        };
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
    use crate::{ChunkKind, NewChunk};

    fn memorya() -> (tempfile::TempDir, Engram) {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        (dir, e)
    }

    #[test]
    fn post_hook_route_returns_404() {
        let (_d, e) = memorya();
        assert_eq!(route(&e, "POST", "/hook").status, 404);
    }

    #[test]
    fn get_root_viewer_still_renders_recent_chunks() {
        let (_d, e) = memorya();
        e.ingest(NewChunk {
            session_id: None,
            kind: ChunkKind::Doc,
            content: "a memorable line in the viewer".into(),
            title: Some("doc".into()),
            file_path: Some("/p/a.md".into()),
            turn_index: None,
            ts: 1,
        })
        .unwrap();

        let resp = route(&e, "GET", "/");

        assert_eq!(resp.content_type, "text/html; charset=utf-8");
        assert!(resp.body.contains("memorya memory"));
        assert!(resp.body.contains("a memorable line in the viewer"));
    }

    #[test]
    fn healthz_still_returns_ok() {
        let (_d, e) = memorya();
        let resp = route(&e, "GET", "/healthz");
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("\"ok\":true"));
    }

    #[test]
    fn viewer_binds_loopback_127_0_0_1_only_in_both_modes() {
        let listener = bind(0).unwrap();
        assert!(
            listener.local_addr().unwrap().ip().is_loopback(),
            "the viewer must bind loopback only"
        );
    }
}
