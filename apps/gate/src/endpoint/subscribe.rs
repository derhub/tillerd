//! The consumer subscribe face: a loopback Unix socket a consumer opens to stream
//! a session's hook events.
//!
//! It reads one [`HookSubscribeRequest`], negotiates the hook-subscription wire
//! version, then streams a ready frame followed by event frames drawn from the
//! session's broadcast channel, recording drop-oldest lag as it observes it.

use std::path::PathBuf;
use std::sync::Arc;

use contracts::HookSubscribeRequest;
use serde_json::json;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;

use crate::endpoint::{read_frame, write_frame};
use crate::subscription::{encode_event, encode_ready, negotiate, Subscriptions};

/// Bind the subscribe face to `socket_path` and serve it until the task is aborted.
pub fn serve(
    socket_path: PathBuf,
    subscriptions: Arc<Subscriptions>,
) -> std::io::Result<JoinHandle<()>> {
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    Ok(tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let subscriptions = subscriptions.clone();
            tokio::spawn(handle_conn(stream, subscriptions));
        }
    }))
}

async fn handle_conn(stream: UnixStream, subscriptions: Arc<Subscriptions>) {
    let (mut rd, mut wr) = stream.into_split();
    let Ok(Some(frame)) = read_frame(&mut rd).await else {
        return;
    };
    let request: HookSubscribeRequest = match serde_json::from_slice(&frame) {
        Ok(request) => request,
        Err(e) => {
            let _ = write_frame(&mut wr, &encode_error(&format!("malformed request: {e}"))).await;
            return;
        }
    };
    if negotiate(request.wire_version).is_none() {
        let _ = write_frame(&mut wr, &encode_error("unsupported wire version")).await;
        return;
    }

    let mut rx = subscriptions.subscribe(&request.session_id);
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
            Err(RecvError::Lagged(n)) => subscriptions.record_lag(&request.session_id, n),
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
    use contracts::{
        CorrelationId, HookEvent, HookKind, SessionId, HOOK_SUBSCRIPTION_WIRE_VERSION,
    };
    use serde_json::Value;
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

    fn request(session: &str, wire_version: u32) -> Vec<u8> {
        serde_json::to_vec(&HookSubscribeRequest {
            session_id: SessionId(session.into()),
            wire_version,
        })
        .unwrap()
    }

    fn event(correlation: &str) -> HookEvent {
        HookEvent {
            session_id: SessionId("s".into()),
            correlation_id: CorrelationId(correlation.into()),
            ts: 0,
            kind: HookKind::Stop { turn_index: None },
        }
    }

    #[tokio::test]
    async fn negotiates_the_wire_version_then_streams_events() {
        let sock = temp_sock("stream");
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let handle = serve(sock.clone(), subscriptions.clone()).unwrap();

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        write_frame(&mut stream, &request("s", HOOK_SUBSCRIPTION_WIRE_VERSION))
            .await
            .unwrap();

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
        let handle = serve(sock.clone(), subscriptions).unwrap();

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        write_frame(
            &mut stream,
            &request("s", HOOK_SUBSCRIPTION_WIRE_VERSION + 1),
        )
        .await
        .unwrap();

        let value: Value =
            serde_json::from_slice(&read_frame(&mut stream).await.unwrap().unwrap()).unwrap();
        assert_eq!(value["frame"], "error", "an unsupported version is refused");

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }
}
