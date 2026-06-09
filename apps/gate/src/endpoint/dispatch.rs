//! Connection demux for the gate's single socket. Reads the route preamble, applies
//! the one centralized route->credential policy, then hands the connection to its
//! route. A malformed/unknown/unsupported preamble, or a failed credential check,
//! drops the connection before any route runs.

use std::sync::Arc;

use contracts::{Route, RoutePreamble, SessionId};
use tokio::net::UnixStream;

use crate::endpoint::{admin, hook, mcp, read_frame, subscribe, tool};
use crate::registry::SessionRegistry;
use crate::router::Router;
use crate::subscription::{negotiate, Subscriptions};
use crate::Token;

/// The shared faces a dispatched connection may reach.
#[derive(Clone)]
pub struct Faces {
    /// The session registry, for verifying session-token routes and admin mutation.
    pub registry: Arc<SessionRegistry>,
    /// The admin face: admin-token check plus registry mutation.
    pub admin: Arc<admin::Admin>,
    /// The per-session hook-event pub/sub for the subscribe route.
    pub subscriptions: Arc<Subscriptions>,
    /// The middleware router for the hook, tool, and mcp routes.
    pub router: Arc<Router>,
}

/// What a valid preamble authorized: the route plus the credentials its handler needs.
enum Authorized {
    /// Hook, Tool, or Mcp: a verified session and its token.
    Session {
        route: Route,
        session: SessionId,
        token: Token,
    },
    /// Subscribe: a session and the negotiated wire version; no token.
    Subscribe {
        session: SessionId,
        wire_version: u32,
    },
    /// Admin: the admin token was verified; the command names its own session.
    Admin,
}

/// Read one connection's preamble and run its route, or drop the connection if the
/// preamble is malformed, declares an unsupported wire version, names an unknown
/// route, or fails the route's credential check.
pub async fn dispatch(mut stream: UnixStream, faces: Faces) {
    let Ok(Some(frame)) = read_frame(&mut stream).await else {
        return;
    };
    let Ok(preamble) = serde_json::from_slice::<RoutePreamble>(&frame) else {
        return;
    };
    let Some(authorized) = authorize(&faces, &preamble) else {
        return;
    };
    match authorized {
        Authorized::Session {
            route: Route::Hook,
            session,
            token,
        } => hook::serve_conn(stream, faces.router, session, token).await,
        Authorized::Session {
            route: Route::Tool,
            session,
            token,
        } => tool::serve_conn(stream, faces.router, session, token).await,
        Authorized::Session {
            route: Route::Mcp,
            session,
            token,
        } => mcp::serve_conn(stream, faces.router, session, token).await,
        // Only Hook/Tool/Mcp ever produce a Session authorization.
        Authorized::Session { .. } => {}
        Authorized::Subscribe {
            session,
            wire_version,
        } => subscribe::serve_conn(stream, faces.subscriptions, session, wire_version).await,
        Authorized::Admin => admin::serve_conn(stream, faces.admin).await,
    }
}

/// The one route->credential policy: Hook/Tool/Mcp require a verified session token;
/// Admin requires the admin token (a session token can never satisfy it); Subscribe
/// requires none. Every route additionally requires a supported wire version.
fn authorize(faces: &Faces, preamble: &RoutePreamble) -> Option<Authorized> {
    negotiate(preamble.wire_version)?;
    match preamble.route {
        Route::Hook | Route::Tool | Route::Mcp => {
            let session = preamble.session_id.clone()?;
            let token = Token::new(preamble.token.clone()?);
            faces.registry.verify(&session, &token)?;
            Some(Authorized::Session {
                route: preamble.route,
                session,
                token,
            })
        }
        Route::Subscribe => {
            let session = preamble.session_id.clone()?;
            Some(Authorized::Subscribe {
                session,
                wire_version: preamble.wire_version,
            })
        }
        Route::Admin => {
            let token = preamble.token.as_deref()?;
            faces.admin.authenticate(token).then_some(Authorized::Admin)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_adapter::V1Adapter;
    use crate::endpoint::{read_frame, write_frame};
    use crate::middleware::auth::Auth;
    use crate::middleware::fanout::FanOut;
    use crate::middleware::normalize::Normalize;
    use crate::middleware::passthrough::PassThrough;
    use crate::middleware::{seq, Middleware};
    use crate::Kind;
    use contracts::HOOK_SUBSCRIPTION_WIRE_VERSION;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;

    fn faces_with(session: &str, session_token: &str, admin_token: &str) -> Faces {
        let registry = Arc::new(SessionRegistry::new());
        registry.register(SessionId(session.into()), &Token::new(session_token));
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let admin = Arc::new(admin::Admin::new(
            &Token::new(admin_token),
            registry.clone(),
        ));
        let globals = vec![Arc::new(Auth::new(registry.clone())) as Arc<dyn Middleware>];
        let hook_route = seq(vec![
            Arc::new(Normalize::new(Arc::new(V1Adapter))),
            Arc::new(FanOut::new(subscriptions.clone())),
        ]);
        let routes = HashMap::from([
            (Kind::Hook, hook_route),
            (Kind::ToolCall, Arc::new(PassThrough) as Arc<dyn Middleware>),
            (
                Kind::ToolResult,
                Arc::new(PassThrough) as Arc<dyn Middleware>,
            ),
            (Kind::Mcp, Arc::new(PassThrough) as Arc<dyn Middleware>),
        ]);
        let router = Arc::new(Router::new(globals, routes));
        Faces {
            registry,
            admin,
            subscriptions,
            router,
        }
    }

    fn temp_sock(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        PathBuf::from(format!(
            "/tmp/gate-dispatch-{tag}-{}-{nanos}.sock",
            std::process::id()
        ))
    }

    fn serve(sock: &PathBuf, faces: Faces) -> tokio::task::JoinHandle<()> {
        let _ = std::fs::remove_file(sock);
        let listener = tokio::net::UnixListener::bind(sock).unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(dispatch(stream, faces.clone()));
            }
        })
    }

    fn preamble(route: &str, session: Option<&str>, token: Option<&str>) -> Vec<u8> {
        let mut value = json!({ "route": route, "wireVersion": HOOK_SUBSCRIPTION_WIRE_VERSION });
        if let Some(s) = session {
            value["sessionId"] = json!(s);
        }
        if let Some(t) = token {
            value["token"] = json!(t);
        }
        serde_json::to_vec(&value).unwrap()
    }

    #[tokio::test]
    async fn hook_route_reaches_fan_out() {
        let sock = temp_sock("hook");
        let faces = faces_with("s", "secret", "admin");
        let mut rx = faces.subscriptions.subscribe(&SessionId("s".into()));
        let handle = serve(&sock, faces);

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        write_frame(&mut stream, &preamble("hook", Some("s"), Some("secret")))
            .await
            .unwrap();
        write_frame(
            &mut stream,
            &serde_json::to_vec(
                &json!({ "hook_event_name": "Stop", "session_id": "a", "timestamp_ms": 5 }),
            )
            .unwrap(),
        )
        .await
        .unwrap();

        let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("hook fans out")
            .unwrap();
        assert_eq!(event.session_id, SessionId("s".into()));

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn tool_route_responds() {
        let sock = temp_sock("tool");
        let handle = serve(&sock, faces_with("s", "secret", "admin"));

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        write_frame(&mut stream, &preamble("tool", Some("s"), Some("secret")))
            .await
            .unwrap();
        let call = json!({
            "type": "ToolCall",
            "payload": { "sessionId": "s", "correlationId": "c", "toolName": "Bash", "toolInput": {} }
        });
        write_frame(&mut stream, &serde_json::to_vec(&call).unwrap())
            .await
            .unwrap();

        let response = read_frame(&mut stream).await.unwrap().unwrap();
        let value: Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(value["result"], "forward");

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn subscribe_route_sends_ready() {
        let sock = temp_sock("sub");
        let handle = serve(&sock, faces_with("s", "secret", "admin"));

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        write_frame(&mut stream, &preamble("subscribe", Some("s"), None))
            .await
            .unwrap();

        let ready = read_frame(&mut stream).await.unwrap().unwrap();
        let value: Value = serde_json::from_slice(&ready).unwrap();
        assert_eq!(value["frame"], "ready");

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn admin_route_registers_a_session() {
        let sock = temp_sock("admin");
        let faces = faces_with("s", "secret", "admin-secret");
        let registry = faces.registry.clone();
        let handle = serve(&sock, faces);

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        write_frame(&mut stream, &preamble("admin", None, Some("admin-secret")))
            .await
            .unwrap();
        write_frame(
            &mut stream,
            &serde_json::to_vec(
                &json!({ "command": "register", "sessionId": "new", "token": "t" }),
            )
            .unwrap(),
        )
        .await
        .unwrap();

        let response = read_frame(&mut stream).await.unwrap().unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&response).unwrap()["result"],
            "ok"
        );
        assert!(
            registry
                .verify(&SessionId("new".into()), &Token::new("t"))
                .is_some(),
            "the admin route registered the session"
        );

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn a_session_token_cannot_satisfy_the_admin_route() {
        // The centralized policy compares the Admin route's token against the admin
        // token only; a valid per-session token must never admit a mutation.
        let sock = temp_sock("admin-wall");
        let faces = faces_with("s", "secret", "admin-secret");
        let registry = faces.registry.clone();
        let handle = serve(&sock, faces);

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        // Present the valid SESSION token on the Admin route.
        write_frame(&mut stream, &preamble("admin", Some("s"), Some("secret")))
            .await
            .unwrap();
        write_frame(
            &mut stream,
            &serde_json::to_vec(
                &json!({ "command": "register", "sessionId": "intruder", "token": "t" }),
            )
            .unwrap(),
        )
        .await
        .unwrap();

        // The connection is dropped before any command is executed.
        let outcome =
            tokio::time::timeout(Duration::from_millis(300), read_frame(&mut stream)).await;
        assert!(
            matches!(outcome, Ok(Ok(None)) | Ok(Err(_))),
            "the admin route refuses a session token and closes"
        );
        assert!(
            registry
                .verify(&SessionId("intruder".into()), &Token::new("t"))
                .is_none(),
            "a session token never mutates the registry via the admin route"
        );

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn a_malformed_preamble_is_refused() {
        let sock = temp_sock("malformed");
        let handle = serve(&sock, faces_with("s", "secret", "admin"));

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        write_frame(&mut stream, b"{ not a preamble").await.unwrap();

        let outcome =
            tokio::time::timeout(Duration::from_millis(300), read_frame(&mut stream)).await;
        assert!(
            matches!(outcome, Ok(Ok(None)) | Ok(Err(_))),
            "a malformed preamble drops the connection"
        );

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn an_unknown_route_is_refused() {
        let sock = temp_sock("unknown-route");
        let handle = serve(&sock, faces_with("s", "secret", "admin"));

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        let bad = json!({
            "route": "bogus",
            "sessionId": "s",
            "token": "secret",
            "wireVersion": HOOK_SUBSCRIPTION_WIRE_VERSION
        });
        write_frame(&mut stream, &serde_json::to_vec(&bad).unwrap())
            .await
            .unwrap();

        let outcome =
            tokio::time::timeout(Duration::from_millis(300), read_frame(&mut stream)).await;
        assert!(
            matches!(outcome, Ok(Ok(None)) | Ok(Err(_))),
            "an unknown route drops the connection before any face exchange"
        );

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn an_unsupported_wire_version_is_refused() {
        let sock = temp_sock("badwire");
        let handle = serve(&sock, faces_with("s", "secret", "admin"));

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        let bad = json!({
            "route": "hook",
            "sessionId": "s",
            "token": "secret",
            "wireVersion": HOOK_SUBSCRIPTION_WIRE_VERSION + 1
        });
        write_frame(&mut stream, &serde_json::to_vec(&bad).unwrap())
            .await
            .unwrap();

        let outcome =
            tokio::time::timeout(Duration::from_millis(300), read_frame(&mut stream)).await;
        assert!(
            matches!(outcome, Ok(Ok(None)) | Ok(Err(_))),
            "an unsupported wire version drops the connection"
        );

        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }

    #[tokio::test]
    async fn mcp_route_upgrades_after_a_verified_preamble() {
        use rmcp::ServiceExt;
        let sock = temp_sock("mcp");
        let handle = serve(&sock, faces_with("s", "secret", "admin"));

        let mut stream = UnixStream::connect(&sock).await.unwrap();
        write_frame(&mut stream, &preamble("mcp", Some("s"), Some("secret")))
            .await
            .unwrap();
        // After the preamble the stream speaks MCP; a client completes initialize.
        let client = ().serve(stream).await.expect("mcp client initializes after upgrade");
        assert!(client.peer_info().is_some(), "the handshake completed");

        let _ = client.cancel().await;
        handle.abort();
        let _ = std::fs::remove_file(&sock);
    }
}
