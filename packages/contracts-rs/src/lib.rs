//! Shared wire types for the composable tools: the canonical [`HookEvent`], its identifiers,
//! and the gate hook-subscription + tool-route message shapes.
//!
//! Pure contract types — no I/O, no side effects. The encodings here are mirrored by
//! `@athing/sdk` (TypeScript) and pinned by a cross-language golden fixture.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Wire version of the daemon session-event subscription, negotiated per connection.
pub const SESSION_EVENT_WIRE_VERSION: u32 = 1;

/// Wire version of the gate hook-subscription stream, negotiated per connection.
pub const HOOK_SUBSCRIPTION_WIRE_VERSION: u32 = 1;

/// Opaque per-session identifier minted by an orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub String);

/// Opaque id threaded through every message so records join across processes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(pub String);

/// A normalized agent lifecycle event. Flat on the wire:
/// `{ sessionId, correlationId, ts, type, payload }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEvent {
    /// Session this event belongs to.
    pub session_id: SessionId,
    /// Correlation id, assigned on entry and preserved end to end.
    pub correlation_id: CorrelationId,
    /// Event timestamp in epoch milliseconds.
    pub ts: i64,
    /// Discriminant and its typed payload.
    #[serde(flatten)]
    pub kind: HookKind,
}

/// The lifecycle event kinds and their typed payloads. Adjacently tagged on the wire
/// as `{ "type": <Variant>, "payload": { ... } }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum HookKind {
    /// A session started.
    #[serde(rename_all = "camelCase")]
    SessionStart {
        /// Working directory the agent launched in.
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        /// Agent client identifier, when reported.
        #[serde(skip_serializing_if = "Option::is_none")]
        client: Option<String>,
        /// Agent CLI version, when reported.
        #[serde(skip_serializing_if = "Option::is_none")]
        cli_version: Option<String>,
    },
    /// The user submitted a prompt.
    #[serde(rename_all = "camelCase")]
    UserPromptSubmit {
        /// Prompt text.
        content: String,
        /// Monotonic turn index, when known.
        #[serde(skip_serializing_if = "Option::is_none")]
        turn_index: Option<i64>,
    },
    /// A tool finished executing.
    #[serde(rename_all = "camelCase")]
    PostToolUse {
        /// Tool name.
        tool_name: String,
        /// Tool input as raw JSON.
        tool_input: serde_json::Value,
        /// Tool response rendered as text.
        tool_response: String,
        /// Turn index.
        turn_index: i64,
    },
    /// The agent is requesting permission to run a tool.
    #[serde(rename_all = "camelCase")]
    PermissionRequest {
        /// Tool the agent wants to run, when reported.
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        /// Raw request payload.
        request: serde_json::Value,
    },
    /// The agent stopped (turn complete).
    #[serde(rename_all = "camelCase")]
    Stop {
        /// Turn index, when known.
        #[serde(skip_serializing_if = "Option::is_none")]
        turn_index: Option<i64>,
    },
    /// The session ended.
    #[serde(rename_all = "camelCase")]
    SessionEnd {
        /// Reason, when reported.
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

/// A request to subscribe to a session's hook-event stream (gate hook-subscription wire).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSubscribeRequest {
    /// Session to subscribe to.
    pub session_id: SessionId,
    /// Wire version the consumer speaks.
    pub wire_version: u32,
}

/// A tool-route inbound the gate observes and passes through. Adjacently tagged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ToolInbound {
    /// A tool call from the agent.
    #[serde(rename_all = "camelCase")]
    ToolCall {
        /// Session id.
        session_id: SessionId,
        /// Correlation id.
        correlation_id: CorrelationId,
        /// Tool name.
        tool_name: String,
        /// Tool input as raw JSON.
        tool_input: serde_json::Value,
    },
    /// A tool result returning to the agent.
    #[serde(rename_all = "camelCase")]
    ToolResult {
        /// Session id.
        session_id: SessionId,
        /// Correlation id.
        correlation_id: CorrelationId,
        /// Tool name.
        tool_name: String,
        /// Tool response rendered as text.
        tool_response: String,
    },
}
