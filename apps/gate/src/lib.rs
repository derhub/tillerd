#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! The gate's composable middleware core: the context that flows through the
//! onion, the outcome types, and the module tree. The transport faces (hook,
//! tool, subscribe, admin) and the router wire these together.

pub mod agent_adapter;
pub mod endpoint;
pub mod middleware;
pub mod registry;
pub mod router;
pub mod service;
pub mod subscription;

use std::sync::{Arc, Mutex};

use bytes::Bytes;
use contracts::{CorrelationId, HookEvent, SessionId};

/// Which face produced an inbound and which route handles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// An inbound posted by the agent's hook ingress face.
    Hook,
    /// An inbound carrying a tool-call payload from the tool IPC face.
    ToolCall,
    /// An inbound carrying a tool-result payload from the tool IPC face.
    ToolResult,
    /// An inbound carrying an MCP request normalized by the MCP ingress face.
    Mcp,
}

/// A per-session bearer token, compared in constant time against the registry.
#[derive(Debug, Clone)]
pub struct Token(String);

impl Token {
    /// Wrap a string value as a bearer token.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The token's bytes, used for constant-time comparisons.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// Observation fields a downstream layer discovers only after an outer layer has
/// already moved the context into the inner chain. An around-layer clones the
/// shared handle before `next.run` and reads it once the chain returns.
#[derive(Debug, Default)]
pub struct RecordMeta {
    /// The canonical event-type name set by the normalize layer, e.g. `"Stop"`.
    pub event_type: Option<String>,
    /// The number of subscribers the fan-out layer reached.
    pub fanout_n: Option<usize>,
}

/// The context threaded through the middleware onion. It moves into
/// `next.run(ctx)`; copy `session`/`correlation`/`kind` before calling next if
/// they are needed afterward. `record` is a shared back-channel that survives the
/// move so an outer layer can read what inner layers learned.
#[derive(Debug, Clone)]
pub struct Ctx {
    /// The inbound kind, used for routing and observation.
    pub kind: Kind,
    /// The authenticated session this inbound belongs to.
    pub session: SessionId,
    /// The correlation id assigned by the router; unique per inbound.
    pub correlation: CorrelationId,
    /// The bearer token extracted from the inbound, verified by the auth layer.
    pub token: Token,
    /// The raw inbound payload as received from the transport face.
    pub body: Bytes,
    /// The canonical hook event, set by the normalize layer for hook inbounds.
    pub event: Option<HookEvent>,
    /// Shared back-channel for fields written by inner layers and read by outer ones.
    pub record: Arc<Mutex<RecordMeta>>,
}

/// A successful terminal outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outbound {
    /// The inbound was consumed and acknowledged; no payload to forward.
    Accepted,
    /// The inbound's payload is forwarded unchanged to the caller.
    Forward(Bytes),
}

/// A rejected outcome. `Denied` is reserved for a later allow-policy; v1 is
/// allow-all and never produces it.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Reject {
    /// The inbound's token did not authenticate against the session registry.
    #[error("unauthenticated")]
    Unauthenticated,
    /// The inbound was structurally invalid (e.g. malformed JSON, missing field).
    #[error("invalid: {0}")]
    Invalid(String),
    /// The inbound was authenticated but denied by an allow policy.
    #[error("denied: {0}")]
    Denied(String),
}

/// The result every middleware and the router produce.
pub type Flow = Result<Outbound, Reject>;
