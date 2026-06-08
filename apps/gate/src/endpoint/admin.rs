//! The admin face: register and deregister sessions on a separate, authenticated
//! loopback Unix socket.
//!
//! The admin token is distinct from any session token and is compared in constant
//! time over the full token bytes. An unauthenticated or malformed request never
//! mutates the registry. This is the only face that can mutate the session
//! registry: the hook and tool faces have no path to it.

use std::path::PathBuf;
use std::sync::Arc;

use contracts::SessionId;
use serde::Deserialize;
use serde_json::{json, Value};
use subtle::ConstantTimeEq;
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinHandle;

use crate::endpoint::{read_frame, write_frame};
use crate::registry::SessionRegistry;
use crate::Token;

/// An admin request: the admin token plus the registry mutation it authorizes.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdminRequest {
    admin_token: String,
    request: AdminCommand,
}

/// A registry mutation, internally tagged by `command`.
#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "camelCase")]
enum AdminCommand {
    #[serde(rename_all = "camelCase")]
    Register {
        session_id: SessionId,
        token: String,
    },
    #[serde(rename_all = "camelCase")]
    Deregister { session_id: SessionId },
}

/// The admin face: full admin-token bytes and the registry it owns.
pub struct Admin {
    token_bytes: Box<[u8]>,
    registry: Arc<SessionRegistry>,
}

impl Admin {
    /// Build the admin face for an admin token, distinct from any session token.
    pub fn new(admin_token: &Token, registry: Arc<SessionRegistry>) -> Self {
        Self {
            token_bytes: admin_token.as_bytes().into(),
            registry,
        }
    }

    /// Constant-time compare the provided token against the stored admin token.
    /// A length mismatch is an unconditional reject.
    fn authenticate(&self, provided: &str) -> bool {
        let provided = provided.as_bytes();
        if self.token_bytes.len() != provided.len() {
            return false;
        }
        bool::from(self.token_bytes.ct_eq(provided))
    }

    /// Apply one admin frame: authenticate, then mutate the registry. A malformed
    /// or unauthenticated request returns its outcome and never mutates.
    pub(crate) fn apply(&self, frame: &[u8]) -> Vec<u8> {
        let request = match serde_json::from_slice::<AdminRequest>(frame) {
            Ok(request) => request,
            Err(e) => return encode(&json!({ "result": "invalid", "reason": e.to_string() })),
        };
        if !self.authenticate(&request.admin_token) {
            return encode(&json!({ "result": "unauthenticated" }));
        }
        match request.request {
            AdminCommand::Register { session_id, token } => {
                self.registry.register(session_id, &Token::new(token));
            }
            AdminCommand::Deregister { session_id } => {
                self.registry.deregister(&session_id);
            }
        }
        encode(&json!({ "result": "ok" }))
    }
}

/// Bind the admin face to `socket_path` and serve it until the task is aborted.
pub fn serve(socket_path: PathBuf, admin: Arc<Admin>) -> std::io::Result<JoinHandle<()>> {
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    Ok(tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let admin = admin.clone();
            tokio::spawn(handle_conn(stream, admin));
        }
    }))
}

async fn handle_conn(stream: UnixStream, admin: Arc<Admin>) {
    let (mut rd, mut wr) = stream.into_split();
    while let Ok(Some(frame)) = read_frame(&mut rd).await {
        let response = admin.apply(&frame);
        if write_frame(&mut wr, &response).await.is_err() {
            break;
        }
    }
}

fn encode(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).expect("admin response encodes")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admin_for(admin_token: &str) -> (Admin, Arc<SessionRegistry>) {
        let registry = Arc::new(SessionRegistry::new());
        (
            Admin::new(&Token::new(admin_token), registry.clone()),
            registry,
        )
    }

    fn register_frame(admin_token: &str, session: &str, token: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "adminToken": admin_token,
            "request": { "command": "register", "sessionId": session, "token": token },
        }))
        .unwrap()
    }

    fn deregister_frame(admin_token: &str, session: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "adminToken": admin_token,
            "request": { "command": "deregister", "sessionId": session },
        }))
        .unwrap()
    }

    fn result(response: &[u8]) -> String {
        let value: Value = serde_json::from_slice(response).unwrap();
        value["result"].as_str().unwrap().to_string()
    }

    fn session(id: &str) -> SessionId {
        SessionId(id.into())
    }

    #[test]
    fn register_command_adds_a_session_to_the_registry() {
        let (admin, registry) = admin_for("admin-secret");

        let response = admin.apply(&register_frame("admin-secret", "s1", "sess-token"));

        assert_eq!(result(&response), "ok");
        assert!(
            registry
                .verify(&session("s1"), &Token::new("sess-token"))
                .is_some(),
            "the session is now registered"
        );
    }

    #[test]
    fn deregister_command_removes_a_session() {
        let (admin, registry) = admin_for("admin-secret");
        admin.apply(&register_frame("admin-secret", "s1", "sess-token"));

        let response = admin.apply(&deregister_frame("admin-secret", "s1"));

        assert_eq!(result(&response), "ok");
        assert!(
            registry
                .verify(&session("s1"), &Token::new("sess-token"))
                .is_none(),
            "the session is no longer registered"
        );
    }

    #[test]
    fn rejects_a_request_with_the_wrong_admin_token() {
        let (admin, registry) = admin_for("admin-secret");

        let response = admin.apply(&register_frame("not-the-admin-token", "s1", "sess-token"));

        assert_eq!(result(&response), "unauthenticated");
        assert!(
            registry
                .verify(&session("s1"), &Token::new("sess-token"))
                .is_none(),
            "a wrong admin token never mutates the registry"
        );
    }

    #[test]
    fn authenticates_the_admin_token_via_constant_time_comparison() {
        let (admin, _registry) = admin_for("admin-secret");

        assert_eq!(
            result(&admin.apply(&register_frame("admin-secret", "s1", "t"))),
            "ok"
        );
        assert_eq!(
            result(&admin.apply(&register_frame("admin-secrer", "s2", "t"))),
            "unauthenticated",
            "a token differing by one byte is refused"
        );
    }

    #[test]
    fn rejects_a_malformed_admin_frame() {
        let (admin, _registry) = admin_for("admin-secret");

        assert_eq!(result(&admin.apply(b"{ not json")), "invalid");
    }

    // Face isolation: the hook and tool faces never reach session registration.

    fn tool_frame(token: &str, inbound: &Value) -> Vec<u8> {
        serde_json::to_vec(&json!({ "token": token, "inbound": inbound })).unwrap()
    }

    #[tokio::test]
    async fn the_tool_face_cannot_register_a_session() {
        use crate::endpoint::tool;
        use crate::middleware::auth::Auth;
        use crate::middleware::passthrough::PassThrough;
        use crate::middleware::Middleware;
        use crate::router::Router;
        use crate::Kind;
        use std::collections::HashMap;

        let registry = Arc::new(SessionRegistry::new());
        let globals = vec![Arc::new(Auth::new(registry.clone())) as Arc<dyn Middleware>];
        let routes = HashMap::from([
            (Kind::ToolCall, Arc::new(PassThrough) as Arc<dyn Middleware>),
            (
                Kind::ToolResult,
                Arc::new(PassThrough) as Arc<dyn Middleware>,
            ),
        ]);
        let router = Router::new(globals, routes);

        // An admin-register-shaped payload is meaningless to the tool face.
        let response = tool::process(
            &tool_frame(
                "x",
                &json!({ "command": "register", "sessionId": "intruder", "token": "t" }),
            ),
            &router,
        )
        .await;

        let value: Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(value["result"], "reject");
        assert!(
            registry
                .verify(&session("intruder"), &Token::new("t"))
                .is_none(),
            "the tool face cannot create a registry entry"
        );
    }

    #[tokio::test]
    async fn the_hook_route_cannot_register_a_session() {
        use crate::agent_adapter::V1Adapter;
        use crate::middleware::auth::Auth;
        use crate::middleware::fanout::FanOut;
        use crate::middleware::normalize::Normalize;
        use crate::middleware::{seq, Middleware};
        use crate::router::{Inbound, Router};
        use crate::subscription::Subscriptions;
        use crate::{Kind, Reject};
        use bytes::Bytes;
        use std::collections::HashMap;

        let registry = Arc::new(SessionRegistry::new());
        registry.register(session("victim"), &Token::new("vt"));
        let globals = vec![Arc::new(Auth::new(registry.clone())) as Arc<dyn Middleware>];
        let route = seq(vec![
            Arc::new(Normalize::new(Arc::new(V1Adapter))),
            Arc::new(FanOut::new(Arc::new(Subscriptions::with_capacity(8)))),
        ]);
        let router = Router::new(globals, HashMap::from([(Kind::Hook, route)]));

        // A register-shaped body is not a recognized hook, so it is rejected.
        let flow = router
            .handle(Inbound {
                kind: Kind::Hook,
                session: session("victim"),
                correlation: None,
                token: Token::new("vt"),
                body: Bytes::from_static(
                    br#"{"command":"register","sessionId":"intruder","token":"t"}"#,
                ),
            })
            .await;

        assert!(matches!(flow, Err(Reject::Invalid(_))));
        assert!(
            registry
                .verify(&session("intruder"), &Token::new("t"))
                .is_none(),
            "the hook face cannot create a registry entry"
        );
    }

    #[tokio::test]
    async fn the_tool_face_cannot_publish_a_hook_event() {
        use crate::agent_adapter::V1Adapter;
        use crate::endpoint::tool;
        use crate::middleware::auth::Auth;
        use crate::middleware::fanout::FanOut;
        use crate::middleware::normalize::Normalize;
        use crate::middleware::passthrough::PassThrough;
        use crate::middleware::{seq, Middleware};
        use crate::router::Router;
        use crate::subscription::Subscriptions;
        use crate::Kind;
        use std::collections::HashMap;
        use tokio::sync::broadcast::error::TryRecvError;

        let registry = Arc::new(SessionRegistry::new());
        registry.register(session("s"), &Token::new("t"));
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let mut rx = subscriptions.subscribe(&session("s"));
        let globals = vec![Arc::new(Auth::new(registry)) as Arc<dyn Middleware>];
        let hook_route = seq(vec![
            Arc::new(Normalize::new(Arc::new(V1Adapter))),
            Arc::new(FanOut::new(subscriptions.clone())),
        ]);
        let routes = HashMap::from([
            (Kind::Hook, hook_route),
            (Kind::ToolCall, Arc::new(PassThrough) as Arc<dyn Middleware>),
        ]);
        let router = Router::new(globals, routes);

        let response = tool::process(
            &tool_frame(
                "t",
                &json!({
                    "type": "ToolCall",
                    "payload": {
                        "sessionId": "s",
                        "correlationId": "c",
                        "toolName": "Bash",
                        "toolInput": { "command": "ls" },
                    },
                }),
            ),
            &router,
        )
        .await;

        let value: Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(value["result"], "forward", "the tool call is forwarded");
        assert!(
            matches!(rx.try_recv(), Err(TryRecvError::Empty)),
            "a tool call never publishes to hook subscribers"
        );
    }
}
