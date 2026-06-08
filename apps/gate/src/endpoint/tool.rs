//! The tool-route IPC face: a loopback Unix socket carrying length-prefixed tool
//! inbounds.
//!
//! Each request pairs a session token with a [`ToolInbound`]; the face routes the
//! tool call or result through the router (auth then pass-through) and returns the
//! forwarded payload or the rejection. It only ever produces `ToolCall`/
//! `ToolResult` kinds, so it can never publish a hook event.

use std::path::PathBuf;
use std::sync::Arc;

use bytes::Bytes;
use contracts::ToolInbound;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinHandle;

use crate::endpoint::{read_frame, write_frame};
use crate::router::{Inbound, Router};
use crate::{Flow, Kind, Outbound, Token};

/// A tool-route request: the session token plus the tool inbound it authorizes.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolRequest {
    token: String,
    inbound: ToolInbound,
}

/// Bind the tool face to `socket_path` and serve it until the task is aborted.
pub fn serve(socket_path: PathBuf, router: Arc<Router>) -> std::io::Result<JoinHandle<()>> {
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    Ok(tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let router = router.clone();
            tokio::spawn(handle_conn(stream, router));
        }
    }))
}

async fn handle_conn(stream: UnixStream, router: Arc<Router>) {
    let (mut rd, mut wr) = stream.into_split();
    while let Ok(Some(frame)) = read_frame(&mut rd).await {
        let response = process(&frame, &router).await;
        if write_frame(&mut wr, &response).await.is_err() {
            break;
        }
    }
}

/// Process one tool frame: route it and encode the response. A malformed frame is
/// rejected without ever reaching the router.
pub(crate) async fn process(frame: &[u8], router: &Router) -> Vec<u8> {
    match serde_json::from_slice::<ToolRequest>(frame) {
        Ok(request) => encode_response(&router.handle(to_inbound(request)).await),
        Err(e) => encode_reject(&format!("malformed tool inbound: {e}")),
    }
}

fn to_inbound(request: ToolRequest) -> Inbound {
    let (kind, session, correlation) = match &request.inbound {
        ToolInbound::ToolCall {
            session_id,
            correlation_id,
            ..
        } => (Kind::ToolCall, session_id.clone(), correlation_id.clone()),
        ToolInbound::ToolResult {
            session_id,
            correlation_id,
            ..
        } => (Kind::ToolResult, session_id.clone(), correlation_id.clone()),
    };
    let body = Bytes::from(serde_json::to_vec(&request.inbound).expect("tool inbound re-encodes"));
    Inbound {
        kind,
        session,
        correlation: Some(correlation),
        token: Token::new(request.token),
        body,
    }
}

fn encode_response(flow: &Flow) -> Vec<u8> {
    let value = match flow {
        Ok(Outbound::Forward(body)) => json!({
            "result": "forward",
            "inbound": serde_json::from_slice::<Value>(body).unwrap_or(Value::Null),
        }),
        Ok(Outbound::Accepted) => json!({ "result": "accepted" }),
        Err(reject) => json!({ "result": "reject", "reason": reject.to_string() }),
    };
    serde_json::to_vec(&value).expect("tool response encodes")
}

fn encode_reject(reason: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({ "result": "reject", "reason": reason })).expect("reject encodes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::auth::Auth;
    use crate::middleware::passthrough::PassThrough;
    use crate::middleware::Middleware;
    use crate::registry::SessionRegistry;
    use contracts::{CorrelationId, SessionId};
    use std::collections::HashMap;

    fn registry_with(session: &str, token: &str) -> Arc<SessionRegistry> {
        let registry = Arc::new(SessionRegistry::new());
        registry.register(SessionId(session.into()), &Token::new(token));
        registry
    }

    fn tool_router(registry: Arc<SessionRegistry>) -> Arc<Router> {
        let globals = vec![Arc::new(Auth::new(registry)) as Arc<dyn Middleware>];
        let routes = HashMap::from([
            (Kind::ToolCall, Arc::new(PassThrough) as Arc<dyn Middleware>),
            (
                Kind::ToolResult,
                Arc::new(PassThrough) as Arc<dyn Middleware>,
            ),
        ]);
        Arc::new(Router::new(globals, routes))
    }

    fn tool_call(session: &str) -> ToolInbound {
        ToolInbound::ToolCall {
            session_id: SessionId(session.into()),
            correlation_id: CorrelationId("c".into()),
            tool_name: "Bash".into(),
            tool_input: json!({ "command": "ls" }),
        }
    }

    fn frame(token: &str, inbound: &ToolInbound) -> Vec<u8> {
        serde_json::to_vec(&json!({ "token": token, "inbound": inbound })).unwrap()
    }

    #[tokio::test]
    async fn forwards_an_authenticated_tool_call() {
        let router = tool_router(registry_with("s", "secret"));

        let response = process(&frame("secret", &tool_call("s")), &router).await;

        let value: Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(value["result"], "forward");
        let forwarded: ToolInbound = serde_json::from_value(value["inbound"].clone()).unwrap();
        assert_eq!(
            forwarded,
            tool_call("s"),
            "the tool inbound is forwarded unchanged"
        );
    }

    #[tokio::test]
    async fn rejects_a_malformed_frame_without_routing() {
        let router = tool_router(registry_with("s", "secret"));

        let response = process(b"{ not json", &router).await;

        let value: Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(value["result"], "reject");
    }

    #[tokio::test]
    async fn rejects_an_unauthenticated_tool_call() {
        let router = tool_router(registry_with("s", "secret"));

        let response = process(&frame("wrong", &tool_call("s")), &router).await;

        let value: Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(value["result"], "reject");
        assert_eq!(value["reason"], "unauthenticated");
    }

    fn temp_sock(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        PathBuf::from(format!(
            "/tmp/gate-tool-{tag}-{}-{nanos}.sock",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn serves_tool_inbounds_over_a_loopback_unix_socket() {
        let sock = temp_sock("ipc");
        let router = tool_router(registry_with("s", "secret"));
        let handle = serve(sock.clone(), router).unwrap();

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        write_frame(&mut stream, &frame("secret", &tool_call("s")))
            .await
            .unwrap();
        let response = read_frame(&mut stream).await.unwrap().unwrap();

        let value: Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(value["result"], "forward");

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }
}
