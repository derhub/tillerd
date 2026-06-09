//! Session registry: constant-time token verification.

use std::collections::HashMap;
use std::sync::Mutex;

use contracts::SessionId;
use subtle::ConstantTimeEq;

use crate::Token;

/// The access policy recorded for a session. v1 is allow-all; richer policies
/// land here without changing the `Flow` contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowPolicy {
    /// All routes are permitted; the v1 default policy.
    All,
}

struct Entry {
    token_bytes: Box<[u8]>,
    allow_policy: AllowPolicy,
}

/// The in-memory map of `sessionId -> {tokenBytes, allowPolicy}`.
#[derive(Default)]
pub struct SessionRegistry {
    entries: Mutex<HashMap<SessionId, Entry>>,
}

impl SessionRegistry {
    /// Build an empty session registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a session's token under the allow-all policy.
    pub fn register(&self, session: SessionId, token: &Token) {
        let entry = Entry {
            token_bytes: token.as_bytes().into(),
            allow_policy: AllowPolicy::All,
        };
        self.entries
            .lock()
            .expect("session registry mutex poisoned")
            .insert(session, entry);
    }

    /// Remove a session; later inbounds for it fail authentication.
    pub fn deregister(&self, session: &SessionId) {
        self.entries
            .lock()
            .expect("session registry mutex poisoned")
            .remove(session);
    }

    /// Remove every session. Run at shutdown so no in-flight inbound authenticates
    /// while the gate drains.
    pub fn clear(&self) {
        self.entries
            .lock()
            .expect("session registry mutex poisoned")
            .clear();
    }

    /// Constant-time-verify a token against the session's stored bytes,
    /// returning the session's policy on a match.
    ///
    /// Lengths must match for `ct_eq` to be meaningful; a length mismatch is an
    /// unconditional reject, not a timing oracle.
    pub fn verify(&self, session: &SessionId, token: &Token) -> Option<AllowPolicy> {
        let guard = self
            .entries
            .lock()
            .expect("session registry mutex poisoned");
        let entry = guard.get(session)?;
        let provided = token.as_bytes();
        if entry.token_bytes.len() != provided.len() {
            return None;
        }
        bool::from(entry.token_bytes.ct_eq(provided)).then(|| entry.allow_policy.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str) -> SessionId {
        SessionId(id.into())
    }

    #[test]
    fn verifies_a_registered_token() {
        let registry = SessionRegistry::new();
        registry.register(session("s1"), &Token::new("secret"));

        assert_eq!(
            registry.verify(&session("s1"), &Token::new("secret")),
            Some(AllowPolicy::All)
        );
    }

    #[test]
    fn rejects_an_unregistered_session() {
        let registry = SessionRegistry::new();

        assert_eq!(
            registry.verify(&session("ghost"), &Token::new("secret")),
            None
        );
    }

    #[test]
    fn rejects_a_mismatched_token() {
        let registry = SessionRegistry::new();
        registry.register(session("s1"), &Token::new("secret"));

        assert_eq!(registry.verify(&session("s1"), &Token::new("wrong")), None);
    }

    #[test]
    fn deregister_removes_the_session() {
        let registry = SessionRegistry::new();
        registry.register(session("s1"), &Token::new("secret"));

        registry.deregister(&session("s1"));

        assert_eq!(registry.verify(&session("s1"), &Token::new("secret")), None);
    }

    #[test]
    fn clear_removes_every_session() {
        let registry = SessionRegistry::new();
        registry.register(session("s1"), &Token::new("a"));
        registry.register(session("s2"), &Token::new("b"));

        registry.clear();

        assert_eq!(registry.verify(&session("s1"), &Token::new("a")), None);
        assert_eq!(registry.verify(&session("s2"), &Token::new("b")), None);
    }

    #[test]
    fn rejects_a_token_differing_only_in_the_last_byte() {
        let registry = SessionRegistry::new();
        registry.register(
            session("s1"),
            &Token::new("abcdefghijklmnopqrstuvwxyz123456"),
        );

        assert_eq!(
            registry.verify(
                &session("s1"),
                &Token::new("abcdefghijklmnopqrstuvwxyz123457")
            ),
            None
        );
    }
}
