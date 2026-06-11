//! Hook callback client. The agent execs this on each lifecycle event with the
//! payload on stdin; it opens the gate's single socket on the `Hook` route and frames
//! the payload, never blocking or failing the agent.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use contracts::framing::encode_frame;
use contracts::{Route, RoutePreamble, SessionId, HOOK_SUBSCRIPTION_WIRE_VERSION};

const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

fn main() {
    // Fire-and-forget: every failure exits 0 and forwards nothing, so a missing
    // or slow gate never stalls or fails the agent's hook step.
    let _ = run();
}

fn run() -> Option<()> {
    let session_id = non_empty_env("TILLERD_SESSION_ID")?;
    let token = non_empty_env("TILLERD_SESSION_TOKEN")?;
    let socket = tillerd_paths::gate_socket();

    let mut payload = Vec::new();
    std::io::stdin().read_to_end(&mut payload).ok()?;

    send(&socket, &SessionId(session_id), &token, &payload)
}

/// Open the gate socket on the `Hook` route and forward the payload: one preamble
/// frame carrying the session id and token, then one frame of the raw lifecycle
/// payload. Returns `None` on any failure (fire-and-forget).
fn send(socket: &Path, session_id: &SessionId, token: &str, payload: &[u8]) -> Option<()> {
    // Forward only well-formed JSON; the gate's normalize step parses it downstream.
    let _: serde_json::Value = serde_json::from_slice(payload).ok()?;
    let preamble = RoutePreamble {
        route: Route::Hook,
        session_id: Some(session_id.clone()),
        token: Some(token.to_string()),
        wire_version: HOOK_SUBSCRIPTION_WIRE_VERSION,
    };

    let mut stream = UnixStream::connect(socket).ok()?;
    stream.set_write_timeout(Some(WRITE_TIMEOUT)).ok()?;
    stream
        .write_all(&encode_frame(&serde_json::to_vec(&preamble).ok()?))
        .ok()?;
    stream.write_all(&encode_frame(payload)).ok()?;
    stream.flush().ok()
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::framing::FrameDecoder;
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::thread;

    fn temp_sock(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("gn-{tag}-{}.sock", std::process::id()))
    }

    #[test]
    fn send_writes_a_hook_preamble_then_the_raw_payload() {
        let sock = temp_sock("ok");
        let _ = std::fs::remove_file(&sock);
        let listener = UnixListener::bind(&sock).unwrap();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut buf = Vec::new();
            conn.read_to_end(&mut buf).unwrap();
            tx.send(buf).unwrap();
        });

        send(
            &sock,
            &SessionId("s1".into()),
            "tok",
            br#"{"hook_event_name":"Stop"}"#,
        )
        .expect("send succeeds against a live socket");

        let bytes = rx.recv().unwrap();
        let frames = FrameDecoder::new().push(&bytes).unwrap();
        assert_eq!(frames.len(), 2, "a preamble frame then a payload frame");

        let preamble: RoutePreamble = serde_json::from_slice(&frames[0].payload).unwrap();
        assert_eq!(preamble.route, Route::Hook);
        assert_eq!(preamble.session_id, Some(SessionId("s1".into())));
        assert_eq!(preamble.token.as_deref(), Some("tok"));

        let payload: serde_json::Value = serde_json::from_slice(&frames[1].payload).unwrap();
        assert_eq!(payload, serde_json::json!({ "hook_event_name": "Stop" }));
        let _ = std::fs::remove_file(&sock);
    }

    #[test]
    fn send_returns_none_when_the_socket_is_absent() {
        let result = send(
            &temp_sock("missing"),
            &SessionId("s1".into()),
            "tok",
            br#"{"hook_event_name":"Stop"}"#,
        );
        assert!(result.is_none(), "an absent socket forwards nothing");
    }

    #[test]
    fn send_returns_none_when_the_payload_is_not_json() {
        let sock = temp_sock("badjson");
        let _ = std::fs::remove_file(&sock);
        let _listener = UnixListener::bind(&sock).unwrap();
        let result = send(&sock, &SessionId("s1".into()), "tok", b"not json");
        assert!(result.is_none(), "a non-JSON payload forwards nothing");
        let _ = std::fs::remove_file(&sock);
    }
}
