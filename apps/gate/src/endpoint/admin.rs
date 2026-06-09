//! Admin route of the gate's single socket: the only face that mutates the session
//! registry. The route preamble's admin token is verified by the demux (constant-time,
//! distinct from any session token); a session token cannot reach this route. Each
//! subsequent frame is one bare `AdminCommand`.

use std::sync::Arc;

use contracts::SessionId;
use serde::Deserialize;
use serde_json::{json, Value};
use subtle::ConstantTimeEq;
use tokio::net::UnixStream;

use crate::endpoint::{read_frame, write_frame};
use crate::registry::SessionRegistry;
use crate::Token;

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
    /// A length mismatch is an unconditional reject. The demux calls this to admit
    /// an `Admin`-route connection before any command is read.
    pub(crate) fn authenticate(&self, provided: &str) -> bool {
        let provided = provided.as_bytes();
        if self.token_bytes.len() != provided.len() {
            return false;
        }
        bool::from(self.token_bytes.ct_eq(provided))
    }

    /// Execute one bare `AdminCommand` frame: mutate the registry and encode the
    /// outcome. The admin token was already verified by the demux; a malformed
    /// frame never mutates.
    pub(crate) fn execute(&self, frame: &[u8]) -> Vec<u8> {
        let command = match serde_json::from_slice::<AdminCommand>(frame) {
            Ok(command) => command,
            Err(e) => return encode(&json!({ "result": "invalid", "reason": e.to_string() })),
        };
        match command {
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

/// Serve one admin connection whose preamble already admitted the admin token:
/// a request/response loop over bare `AdminCommand` frames.
pub async fn serve_conn(stream: UnixStream, admin: Arc<Admin>) {
    let (mut rd, mut wr) = stream.into_split();
    while let Ok(Some(frame)) = read_frame(&mut rd).await {
        let response = admin.execute(&frame);
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

    fn register_frame(session: &str, token: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({ "command": "register", "sessionId": session, "token": token }))
            .unwrap()
    }

    fn deregister_frame(session: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({ "command": "deregister", "sessionId": session })).unwrap()
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

        let response = admin.execute(&register_frame("s1", "sess-token"));

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
        admin.execute(&register_frame("s1", "sess-token"));

        let response = admin.execute(&deregister_frame("s1"));

        assert_eq!(result(&response), "ok");
        assert!(
            registry
                .verify(&session("s1"), &Token::new("sess-token"))
                .is_none(),
            "the session is no longer registered"
        );
    }

    #[test]
    fn rejects_a_malformed_admin_frame() {
        let (admin, _registry) = admin_for("admin-secret");

        assert_eq!(result(&admin.execute(b"{ not json")), "invalid");
    }

    #[test]
    fn authenticate_accepts_the_admin_token_and_refuses_others() {
        let (admin, _registry) = admin_for("admin-secret");

        assert!(
            admin.authenticate("admin-secret"),
            "the admin token is accepted"
        );
        assert!(
            !admin.authenticate("admin-secrer"),
            "a token differing by one byte is refused"
        );
        assert!(
            !admin.authenticate("admin-secret-longer"),
            "a length mismatch is refused"
        );
    }
}
