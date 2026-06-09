//! MCP stdio server: recall, expand, entity tools.

use crate::Engram;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};

/// Tool descriptors advertised via `tools/list`.
fn tool_list() -> Value {
    json!([
        {
            "name": "recall",
            "description": "Hybrid search over captured sessions and project docs.",
            "inputSchema": {
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"]
            }
        },
        {
            "name": "expand",
            "description": "Return the full content of a chunk by id.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"]
            }
        },
        {
            "name": "entity",
            "description": "List currently-valid facts for a named entity.",
            "inputSchema": {
                "type": "object",
                "properties": { "name": { "type": "string" } },
                "required": ["name"]
            }
        }
    ])
}

fn text_result(text: String) -> Value {
    json!({ "content": [ { "type": "text", "text": text } ] })
}

fn ok(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn err(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Handle one JSON-RPC request and produce a response (or `None` for
/// notifications, which take no reply).
pub fn handle_request(memorya: &Engram, req: &Value, now: i64) -> Option<Value> {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

    // Notifications (no id) get no response.
    let id = id?;

    match method {
        "initialize" => Some(ok(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "memorya", "version": env!("CARGO_PKG_VERSION") }
            }),
        )),
        "tools/list" => Some(ok(id, json!({ "tools": tool_list() }))),
        "tools/call" => {
            let params = req.get("params").cloned().unwrap_or(Value::Null);
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            match call_tool(memorya, name, &args, now) {
                Ok(result) => Some(ok(id, result)),
                Err(e) => Some(err(id, -32000, &e.to_string())),
            }
        }
        _ => Some(err(id, -32601, "method not found")),
    }
}

fn call_tool(memorya: &Engram, name: &str, args: &Value, now: i64) -> anyhow::Result<Value> {
    match name {
        "recall" => {
            let q = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            memorya.embed_pending(10_000)?;
            // Return full content of the top hits inline, so the agent does not
            // need a second `expand` round-trip.
            let results = memorya.search(q, now, 5)?;
            let text = if results.is_empty() {
                "No confident match. Consider searching the archive.".to_string()
            } else {
                results
                    .iter()
                    .map(|r| format!("#{} ({:.3}) {}\n{}", r.id, r.score, r.title, r.content))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            };
            Ok(text_result(text))
        }
        "expand" => {
            let id = args.get("id").and_then(|v| v.as_i64()).unwrap_or(-1);
            let text = memorya
                .expand(id)?
                .unwrap_or_else(|| format!("no chunk #{id}"));
            Ok(text_result(text))
        }
        "entity" => {
            let n = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let text = memorya
                .entity(n)?
                .iter()
                .map(|f| format!("{} = {}", f.predicate, f.value))
                .collect::<Vec<_>>()
                .join("\n");
            Ok(text_result(text))
        }
        other => anyhow::bail!("unknown tool: {other}"),
    }
}

/// Run the stdio server loop: read one JSON-RPC request per line, write one
/// response per line. Each request locks the shared memorya only for its own
/// dispatch, so capture and the embedding worker keep running between requests.
pub fn serve_stdio(memorya: Arc<Mutex<Engram>>, now: impl Fn() -> i64) -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let response = {
            let memorya = memorya.lock().expect("memorya mutex poisoned");
            handle_request(&memorya, &req, now())
        };
        if let Some(resp) = response {
            writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            stdout.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChunkKind, NewChunk};

    fn memorya_with_doc() -> (tempfile::TempDir, Engram) {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        e.ingest(NewChunk {
            session_id: None,
            kind: ChunkKind::Doc,
            content: "the service authenticates with a session token".into(),
            title: Some("auth".into()),
            file_path: Some("/p/a.md".into()),
            turn_index: None,
            ts: 0,
        })
        .unwrap();
        (dir, e)
    }

    #[test]
    fn initialize_reports_server_info() {
        let (_d, e) = memorya_with_doc();
        let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize"});
        let resp = handle_request(&e, &req, 0).unwrap();
        assert_eq!(resp["result"]["serverInfo"]["name"], "memorya");
    }

    #[test]
    fn tools_list_exposes_three_tools() {
        let (_d, e) = memorya_with_doc();
        let req = json!({"jsonrpc":"2.0","id":2,"method":"tools/list"});
        let resp = handle_request(&e, &req, 0).unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn tools_call_recall_returns_text() {
        let (_d, e) = memorya_with_doc();
        let req = json!({
            "jsonrpc":"2.0","id":3,"method":"tools/call",
            "params": { "name": "recall", "arguments": { "query": "session token auth" } }
        });
        let resp = handle_request(&e, &req, 0).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("auth"));
    }

    #[test]
    fn notification_without_id_has_no_response() {
        let (_d, e) = memorya_with_doc();
        let req = json!({"jsonrpc":"2.0","method":"initialized"});
        assert!(handle_request(&e, &req, 0).is_none());
    }

    #[test]
    fn unknown_method_errors() {
        let (_d, e) = memorya_with_doc();
        let req = json!({"jsonrpc":"2.0","id":9,"method":"nope"});
        let resp = handle_request(&e, &req, 0).unwrap();
        assert_eq!(resp["error"]["code"], -32601);
    }
}
