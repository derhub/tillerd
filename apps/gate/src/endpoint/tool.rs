//! Tool-route IPC: the `Tool` route of the gate's single socket. The route preamble
//! admits the connection; each subsequent frame is one bare `ToolInbound`, routed as
//! `ToolCall`/`ToolResult` (never a hook) with one response frame per request.

use std::sync::Arc;

use bytes::Bytes;
use contracts::{SessionId, ToolInbound};
use serde_json::{json, Value};
use tokio::net::UnixStream;

use crate::endpoint::{read_frame, write_frame};
use crate::router::{Inbound, Router};
use crate::{Flow, Kind, Outbound, Token};

/// Serve one tool connection whose preamble already admitted `session`/`token`:
/// a request/response loop over bare `ToolInbound` frames.
pub async fn serve_conn(stream: UnixStream, router: Arc<Router>, session: SessionId, token: Token) {
    let (mut rd, mut wr) = stream.into_split();
    while let Ok(Some(frame)) = read_frame(&mut rd).await {
        let response = process(&frame, &router, &session, &token).await;
        if write_frame(&mut wr, &response).await.is_err() {
            break;
        }
    }
}

/// Process one bare `ToolInbound` frame against the preamble identity: route it and
/// encode the response. A malformed frame is rejected without reaching the router.
pub(crate) async fn process(
    frame: &[u8],
    router: &Router,
    session: &SessionId,
    token: &Token,
) -> Vec<u8> {
    match serde_json::from_slice::<ToolInbound>(frame) {
        Ok(inbound) => encode_response(&router.handle(to_inbound(inbound, session, token)).await),
        Err(e) => encode_reject(&format!("malformed tool inbound: {e}")),
    }
}

/// Build a tool `Inbound` from the payload, attributing it to the preamble-admitted
/// session and token (not any session named inside the payload).
fn to_inbound(inbound: ToolInbound, session: &SessionId, token: &Token) -> Inbound {
    let (kind, correlation) = match &inbound {
        ToolInbound::ToolCall { correlation_id, .. } => (Kind::ToolCall, correlation_id.clone()),
        ToolInbound::ToolResult { correlation_id, .. } => {
            (Kind::ToolResult, correlation_id.clone())
        }
    };
    let body = Bytes::from(serde_json::to_vec(&inbound).expect("tool inbound re-encodes"));
    Inbound {
        kind,
        session: session.clone(),
        correlation: Some(correlation),
        token: token.clone(),
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
    use contracts::CorrelationId;
    use std::collections::HashMap;
    use std::path::PathBuf;

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

    fn frame(inbound: &ToolInbound) -> Vec<u8> {
        serde_json::to_vec(inbound).unwrap()
    }

    #[tokio::test]
    async fn forwards_an_authenticated_tool_call() {
        let router = tool_router(registry_with("s", "secret"));

        let response = process(
            &frame(&tool_call("s")),
            &router,
            &SessionId("s".into()),
            &Token::new("secret"),
        )
        .await;

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

        let response = process(
            b"{ not json",
            &router,
            &SessionId("s".into()),
            &Token::new("secret"),
        )
        .await;

        let value: Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(value["result"], "reject");
    }

    #[tokio::test]
    async fn rejects_a_tool_call_whose_preamble_token_is_wrong() {
        let router = tool_router(registry_with("s", "secret"));

        let response = process(
            &frame(&tool_call("s")),
            &router,
            &SessionId("s".into()),
            &Token::new("wrong"),
        )
        .await;

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
        let _ = std::fs::remove_file(&sock);
        let router = tool_router(registry_with("s", "secret"));
        let listener = tokio::net::UnixListener::bind(&sock).unwrap();
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            serve_conn(stream, router, SessionId("s".into()), Token::new("secret")).await;
        });

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        write_frame(&mut stream, &frame(&tool_call("s")))
            .await
            .unwrap();
        let response = read_frame(&mut stream).await.unwrap().unwrap();

        let value: Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(value["result"], "forward");

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }
}
