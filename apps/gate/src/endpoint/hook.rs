//! Hook ingress: the `Hook` route of the gate's single socket. The route preamble
//! admits the connection; each subsequent frame is one raw lifecycle payload, routed
//! to fan-out with no reply, so a slow subscriber never stalls the producer.

use std::sync::Arc;

use bytes::Bytes;
use contracts::SessionId;
use tokio::net::UnixStream;

use crate::endpoint::read_frame;
use crate::router::{Inbound, Router};
use crate::{Kind, Token};

/// Serve one hook connection whose preamble already admitted `session`/`token`:
/// read raw lifecycle payload frames and route each fire-and-forget, no reply.
pub async fn serve_conn(stream: UnixStream, router: Arc<Router>, session: SessionId, token: Token) {
    let (mut rd, _wr) = stream.into_split();
    while let Ok(Some(frame)) = read_frame(&mut rd).await {
        let inbound = Inbound {
            kind: Kind::Hook,
            session: session.clone(),
            correlation: None,
            token: token.clone(),
            body: Bytes::from(frame),
        };
        let router = router.clone();
        tokio::spawn(async move {
            let _ = router.handle(inbound).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_adapter::V1Adapter;
    use crate::endpoint::write_frame;
    use crate::middleware::auth::Auth;
    use crate::middleware::fanout::FanOut;
    use crate::middleware::normalize::Normalize;
    use crate::middleware::{seq, Middleware};
    use crate::registry::SessionRegistry;
    use crate::subscription::Subscriptions;
    use contracts::HookKind;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;

    fn registry_with(session: &str, token: &str) -> Arc<SessionRegistry> {
        let registry = Arc::new(SessionRegistry::new());
        registry.register(SessionId(session.into()), &Token::new(token));
        registry
    }

    fn hook_router(
        registry: Arc<SessionRegistry>,
        subscriptions: Arc<Subscriptions>,
    ) -> Arc<Router> {
        let globals = vec![Arc::new(Auth::new(registry)) as Arc<dyn Middleware>];
        let route = seq(vec![
            Arc::new(Normalize::new(Arc::new(V1Adapter))),
            Arc::new(FanOut::new(subscriptions)),
        ]);
        Arc::new(Router::new(globals, HashMap::from([(Kind::Hook, route)])))
    }

    fn raw_hook() -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "hook_event_name": "Stop", "session_id": "agent", "timestamp_ms": 5, "turn_index": 2
        }))
        .unwrap()
    }

    fn temp_sock(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        PathBuf::from(format!(
            "/tmp/gate-hook-{tag}-{}-{nanos}.sock",
            std::process::id()
        ))
    }

    async fn serve(
        sock: PathBuf,
        router: Arc<Router>,
        session: &str,
        token: &str,
    ) -> tokio::task::JoinHandle<()> {
        let _ = std::fs::remove_file(&sock);
        let listener = tokio::net::UnixListener::bind(&sock).unwrap();
        let session = SessionId(session.into());
        let token = Token::new(token);
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            serve_conn(stream, router, session, token).await;
        })
    }

    #[tokio::test]
    async fn an_authenticated_hook_payload_fans_out() {
        let sock = temp_sock("fanout");
        let registry = registry_with("s", "secret");
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let mut rx = subscriptions.subscribe(&SessionId("s".into()));
        let router = hook_router(registry, subscriptions.clone());
        let handle = serve(sock.clone(), router, "s", "secret").await;

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        write_frame(&mut stream, &raw_hook()).await.unwrap();

        let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("the authenticated hook fans out")
            .unwrap();
        assert_eq!(
            event.kind,
            HookKind::Stop {
                turn_index: Some(2)
            }
        );

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn a_wrong_session_token_never_fans_out() {
        // The session token is verified by the demux; here the router's Auth global
        // rejects a token that does not match the session, so nothing fans out.
        let sock = temp_sock("wrong");
        let registry = registry_with("s", "secret");
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let mut rx = subscriptions.subscribe(&SessionId("s".into()));
        let router = hook_router(registry, subscriptions.clone());
        let handle = serve(sock.clone(), router, "s", "wrong").await;

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        write_frame(&mut stream, &raw_hook()).await.unwrap();

        assert!(
            tokio::time::timeout(Duration::from_millis(200), rx.recv())
                .await
                .is_err(),
            "a wrong token never reaches fan-out"
        );

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }
}
