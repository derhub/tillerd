//! Subscribe route of the gate's single socket: streams a session's hook events.
//! The route preamble names the session and the wire version; this handler negotiates,
//! acknowledges readiness, then server-pushes events and records drop-oldest lag.

use std::sync::Arc;

use contracts::SessionId;
use serde_json::json;
use tokio::net::UnixStream;
use tokio::sync::broadcast::error::RecvError;

use crate::endpoint::write_frame;
use crate::subscription::{encode_event, encode_ready, negotiate, Subscriptions};

/// Serve one subscribe connection whose preamble named `session` and `wire_version`.
/// Negotiate the wire version, send `ready`, then stream events until the peer closes.
pub async fn serve_conn(
    stream: UnixStream,
    subscriptions: Arc<Subscriptions>,
    session: SessionId,
    wire_version: u32,
) {
    let (_rd, mut wr) = stream.into_split();
    if negotiate(wire_version).is_none() {
        let _ = write_frame(&mut wr, &encode_error("unsupported wire version")).await;
        return;
    }

    let mut rx = subscriptions.subscribe(&session);
    if write_frame(&mut wr, &encode_ready()).await.is_err() {
        return;
    }
    loop {
        match rx.recv().await {
            Ok(event) => {
                if write_frame(&mut wr, &encode_event(&event)).await.is_err() {
                    break;
                }
            }
            Err(RecvError::Lagged(n)) => subscriptions.record_lag(&session, n),
            Err(RecvError::Closed) => break,
        }
    }
}

fn encode_error(reason: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({ "frame": "error", "reason": reason })).expect("error frame encodes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::read_frame;
    use contracts::{CorrelationId, HookEvent, HookKind, HOOK_SUBSCRIPTION_WIRE_VERSION};
    use serde_json::Value;
    use std::path::PathBuf;
    use std::time::Duration;

    fn temp_sock(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        PathBuf::from(format!(
            "/tmp/gate-sub-{tag}-{}-{nanos}.sock",
            std::process::id()
        ))
    }

    fn event(correlation: &str) -> HookEvent {
        HookEvent {
            session_id: SessionId("s".into()),
            correlation_id: CorrelationId(correlation.into()),
            ts: 0,
            kind: HookKind::Stop { turn_index: None },
        }
    }

    async fn serve(
        sock: PathBuf,
        subscriptions: Arc<Subscriptions>,
        session: &str,
        wire_version: u32,
    ) -> tokio::task::JoinHandle<()> {
        let _ = std::fs::remove_file(&sock);
        let listener = tokio::net::UnixListener::bind(&sock).unwrap();
        let session = SessionId(session.into());
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            serve_conn(stream, subscriptions, session, wire_version).await;
        })
    }

    #[tokio::test]
    async fn negotiates_the_wire_version_then_streams_events() {
        let sock = temp_sock("stream");
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let handle = serve(
            sock.clone(),
            subscriptions.clone(),
            "s",
            HOOK_SUBSCRIPTION_WIRE_VERSION,
        )
        .await;

        let mut stream = UnixStream::connect(&sock).await.unwrap();

        let ready: Value =
            serde_json::from_slice(&read_frame(&mut stream).await.unwrap().unwrap()).unwrap();
        assert_eq!(ready["frame"], "ready");
        assert_eq!(ready["wireVersion"], HOOK_SUBSCRIPTION_WIRE_VERSION);

        subscriptions
            .publish(&SessionId("s".into()), event("corr-1"))
            .unwrap();

        let frame = tokio::time::timeout(Duration::from_secs(1), read_frame(&mut stream))
            .await
            .expect("an event is streamed")
            .unwrap()
            .unwrap();
        let value: Value = serde_json::from_slice(&frame).unwrap();
        assert_eq!(value["frame"], "event");
        let delivered: HookEvent = serde_json::from_value(value["event"].clone()).unwrap();
        assert_eq!(delivered, event("corr-1"));

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn rejects_an_unsupported_wire_version() {
        let sock = temp_sock("bad-version");
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let handle = serve(
            sock.clone(),
            subscriptions,
            "s",
            HOOK_SUBSCRIPTION_WIRE_VERSION + 1,
        )
        .await;

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        let value: Value =
            serde_json::from_slice(&read_frame(&mut stream).await.unwrap().unwrap()).unwrap();
        assert_eq!(value["frame"], "error", "an unsupported version is refused");

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }
}
