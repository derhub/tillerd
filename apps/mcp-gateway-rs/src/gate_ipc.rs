//! Gate tool-route IPC client: observe traffic via Forward/Reject responses.
//!
//! The length-prefix framing is the shared [`contracts::framing`] codec; only the
//! async `tokio` stream adapters live here, because the shared codec is
//! runtime-free.
//!
//! R8 fail-open: any failure to reach the gate, a rejection, or a malformed reply
//! is logged and the inbound is forwarded unchanged — the gate is observe-only in
//! v1, so a transient gate must never block a tool call.

use std::path::PathBuf;

use contracts::framing::{encode_frame, HEADER_SIZE, MAX_FRAME_SIZE};
use contracts::{
    CorrelationId, Route, RoutePreamble, SessionId, ToolInbound, HOOK_SUBSCRIPTION_WIRE_VERSION,
};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::UnixStream;

/// Read one length-prefixed frame, or `None` at a clean end of stream.
pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<Option<Vec<u8>>> {
    let mut header = [0u8; HEADER_SIZE];
    match reader.read_exact(&mut header).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_be_bytes(header) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frame exceeds max size",
        ));
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;
    Ok(Some(payload))
}

/// Write one length-prefixed frame and flush it.
pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    payload: &[u8],
) -> std::io::Result<()> {
    writer.write_all(&encode_frame(payload)).await?;
    writer.flush().await
}

/// Why a single tool-route round-trip did not complete; every variant fails open.
#[derive(Debug, thiserror::Error)]
enum GateError {
    #[error("gate socket unreachable: {0}")]
    Connect(#[from] std::io::Error),
    #[error("gate closed the stream before replying")]
    Closed,
    #[error("decode failed: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("malformed gate response")]
    Malformed,
    #[error("gate rejected: {0}")]
    Rejected(String),
}

/// A client of the gate's tool-route face, bound to one session.
#[derive(Debug, Clone)]
pub struct GateToolClient {
    socket_path: PathBuf,
    session_id: SessionId,
    token: String,
}

impl GateToolClient {
    /// Build a client targeting `socket_path` for `session_id`, authorized by `token`.
    pub fn new(socket_path: PathBuf, session_id: SessionId, token: String) -> Self {
        Self {
            socket_path,
            session_id,
            token,
        }
    }

    /// Build a client from the composed-deployment environment, or `None` when the
    /// session identity is absent (standalone, no gate): the gateway then forwards
    /// every tool call without observation.
    pub fn from_env() -> Option<Self> {
        let session_id = std::env::var("ATHING_SESSION_ID")
            .ok()
            .filter(|s| !s.is_empty())?;
        let token = std::env::var("ATHING_SESSION_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())?;
        let base =
            service_host::paths::resolve_base_dir(std::env::var("ATHING_DIR").ok().as_deref());
        Some(Self::new(
            base.join("gate.sock"),
            SessionId(session_id),
            token,
        ))
    }

    /// Route a tool call's input through the gate, returning the input to forward
    /// to the backend: the gate's rewrite on `Forward`, or the original unchanged
    /// on any fail-open path.
    pub async fn route_call(
        &self,
        correlation: &CorrelationId,
        tool_name: &str,
        tool_input: Value,
    ) -> Value {
        let inbound = ToolInbound::ToolCall {
            session_id: self.session_id.clone(),
            correlation_id: correlation.clone(),
            tool_name: tool_name.to_string(),
            tool_input: tool_input.clone(),
        };
        match self.route(inbound).await {
            ToolInbound::ToolCall { tool_input, .. } => tool_input,
            // Defensive: the gate must not turn a call into a result; keep the original.
            ToolInbound::ToolResult { .. } => tool_input,
        }
    }

    /// Report a tool result to the gate (observe-only). Fire-and-forget: a failure
    /// to reach the gate is swallowed by the fail-open path.
    pub async fn observe_result(
        &self,
        correlation: &CorrelationId,
        tool_name: &str,
        tool_response: String,
    ) {
        let inbound = ToolInbound::ToolResult {
            session_id: self.session_id.clone(),
            correlation_id: correlation.clone(),
            tool_name: tool_name.to_string(),
            tool_response,
        };
        let _ = self.route(inbound).await;
    }

    /// Send one inbound to the gate and return what to forward: the gate's reply on
    /// `Forward`, or `inbound` unchanged when the gate is unreachable, rejects, or
    /// replies malformed (R8 fail-open).
    pub async fn route(&self, inbound: ToolInbound) -> ToolInbound {
        match self.try_route(&inbound).await {
            Ok(forwarded) => forwarded,
            Err(err) => {
                tracing::warn!(
                    socket = %self.socket_path.display(),
                    %err,
                    "gate tool route unavailable; forwarding unchanged (fail-open)"
                );
                inbound
            }
        }
    }

    async fn try_route(&self, inbound: &ToolInbound) -> Result<ToolInbound, GateError> {
        // Open the gate's single socket on the Tool route: the preamble carries the
        // session and token, then one bare ToolInbound frame is the request.
        let preamble = RoutePreamble {
            route: Route::Tool,
            session_id: Some(self.session_id.clone()),
            token: Some(self.token.clone()),
            wire_version: HOOK_SUBSCRIPTION_WIRE_VERSION,
        };

        let mut stream = UnixStream::connect(&self.socket_path).await?;
        write_frame(&mut stream, &serde_json::to_vec(&preamble)?).await?;
        write_frame(&mut stream, &serde_json::to_vec(inbound)?).await?;
        let frame = read_frame(&mut stream).await?.ok_or(GateError::Closed)?;

        let value: Value = serde_json::from_slice(&frame)?;
        match value.get("result").and_then(Value::as_str) {
            Some("forward") => {
                let inbound_val = value.get("inbound").ok_or(GateError::Malformed)?;
                Ok(serde_json::from_value(inbound_val.clone())?)
            }
            Some("reject") => Err(GateError::Rejected(
                value
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            )),
            _ => Err(GateError::Malformed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::net::UnixListener;

    /// The (preamble frame, command frame) pair the fake gate captures for assertions.
    type Captured = (Vec<u8>, Vec<u8>);

    // A process-wide counter guarantees distinct socket paths even when parallel
    // tests read the same wall-clock nanosecond.
    static SOCK_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_sock(tag: &str) -> PathBuf {
        let seq = SOCK_SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "gw-gate-ipc-{tag}-{}-{seq}.sock",
            std::process::id()
        ))
    }

    fn tool_call(correlation: &str) -> ToolInbound {
        ToolInbound::ToolCall {
            session_id: SessionId("sess-1".into()),
            correlation_id: CorrelationId(correlation.into()),
            tool_name: "github__create_issue".into(),
            tool_input: json!({ "title": "x" }),
        }
    }

    // A one-shot fake gate: binds synchronously (so the client always connects to a
    // live socket), reads the Tool route preamble then the bare ToolInbound frame,
    // and replies to the inbound with `respond(inbound)`. Both received frames are
    // sent back over the channel for assertions. When `respond` returns `None` the
    // connection is dropped without a reply.
    fn spawn_fake_gate(
        respond: impl Fn(&[u8]) -> Option<Vec<u8>> + Send + 'static,
    ) -> (PathBuf, tokio::sync::oneshot::Receiver<Captured>) {
        let sock = temp_sock("gate");
        let listener = UnixListener::bind(&sock).unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let (mut rd, mut wr) = stream.into_split();
                if let (Ok(Some(preamble)), Ok(Some(inbound))) =
                    (read_frame(&mut rd).await, read_frame(&mut rd).await)
                {
                    let _ = tx.send((preamble, inbound.clone()));
                    if let Some(reply) = respond(&inbound) {
                        let _ = write_frame(&mut wr, &reply).await;
                    }
                }
            }
        });
        (sock, rx)
    }

    fn forward_echo(inbound_frame: &[u8]) -> Option<Vec<u8>> {
        let inbound: Value = serde_json::from_slice(inbound_frame).unwrap();
        Some(serde_json::to_vec(&json!({ "result": "forward", "inbound": inbound })).unwrap())
    }

    #[tokio::test]
    async fn frames_roundtrip_through_encode_and_read() {
        let mut stream = Vec::new();
        stream.extend_from_slice(&encode_frame(b"alpha"));
        let mut reader: &[u8] = &stream;
        assert_eq!(
            read_frame(&mut reader).await.unwrap().as_deref(),
            Some(&b"alpha"[..])
        );
        assert_eq!(read_frame(&mut reader).await.unwrap(), None);
    }

    #[tokio::test]
    async fn sends_the_tool_preamble_and_bare_inbound_to_the_gate() {
        let (sock, received) = spawn_fake_gate(forward_echo);
        let client = GateToolClient::new(sock.clone(), SessionId("sess-1".into()), "secret".into());

        let _ = client.route(tool_call("corr-1")).await;

        let (preamble_frame, inbound_frame) = received.await.unwrap();
        let preamble: RoutePreamble = serde_json::from_slice(&preamble_frame).unwrap();
        assert_eq!(preamble.route, Route::Tool);
        assert_eq!(preamble.session_id, Some(SessionId("sess-1".into())));
        assert_eq!(preamble.token.as_deref(), Some("secret"));
        let inbound: ToolInbound = serde_json::from_slice(&inbound_frame).unwrap();
        assert_eq!(inbound, tool_call("corr-1"));
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn returns_the_forwarded_inbound_on_forward() {
        let (sock, _rx) = spawn_fake_gate(forward_echo);
        let client = GateToolClient::new(sock.clone(), SessionId("sess-1".into()), "t".into());

        let out = client.route(tool_call("corr-1")).await;

        assert_eq!(out, tool_call("corr-1"));
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn preserves_the_correlation_id_through_the_gate() {
        let (sock, _rx) = spawn_fake_gate(forward_echo);
        let client = GateToolClient::new(sock.clone(), SessionId("s".into()), "t".into());

        let out = client.route(tool_call("trace-xyz")).await;

        match out {
            ToolInbound::ToolCall { correlation_id, .. } => {
                assert_eq!(correlation_id, CorrelationId("trace-xyz".into()));
            }
            other => panic!("expected a tool call, got {other:?}"),
        }
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn route_call_applies_the_gate_rewritten_input() {
        let (sock, _rx) = spawn_fake_gate(|inbound_frame| {
            let mut inbound: Value = serde_json::from_slice(inbound_frame).unwrap();
            inbound["payload"]["toolInput"] = json!({ "title": "rewritten" });
            Some(serde_json::to_vec(&json!({ "result": "forward", "inbound": inbound })).unwrap())
        });
        let client = GateToolClient::new(sock.clone(), SessionId("s".into()), "t".into());

        let out = client
            .route_call(
                &CorrelationId("c".into()),
                "github__create_issue",
                json!({ "title": "orig" }),
            )
            .await;

        assert_eq!(out, json!({ "title": "rewritten" }));
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn fails_open_and_forwards_unchanged_when_gate_unreachable() {
        let client = GateToolClient::new(temp_sock("missing"), SessionId("s".into()), "t".into());

        let out = client.route(tool_call("corr-1")).await;

        assert_eq!(out, tool_call("corr-1"));
    }

    #[tokio::test]
    async fn route_call_falls_open_to_the_original_input_when_unreachable() {
        let client = GateToolClient::new(temp_sock("missing2"), SessionId("s".into()), "t".into());

        let out = client
            .route_call(&CorrelationId("c".into()), "x", json!({ "k": 1 }))
            .await;

        assert_eq!(out, json!({ "k": 1 }));
    }

    #[tokio::test]
    async fn fails_open_on_a_reject_reply() {
        let (sock, _rx) = spawn_fake_gate(|_frame| {
            Some(serde_json::to_vec(&json!({ "result": "reject", "reason": "denied: x" })).unwrap())
        });
        let client = GateToolClient::new(sock.clone(), SessionId("s".into()), "t".into());

        let out = client.route(tool_call("corr-1")).await;

        assert_eq!(out, tool_call("corr-1"));
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn fails_open_on_a_malformed_reply() {
        let (sock, _rx) = spawn_fake_gate(|_frame| Some(b"this is not json".to_vec()));
        let client = GateToolClient::new(sock.clone(), SessionId("s".into()), "t".into());

        let out = client.route(tool_call("corr-1")).await;

        assert_eq!(out, tool_call("corr-1"));
        let _ = std::fs::remove_file(&sock);
    }
}
