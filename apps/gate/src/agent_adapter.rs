//! The agent adapter seam: a sync, pure translation from an agent's raw hook
//! payload into the canonical [`HookEvent`]. v1 ships a single in-binary adapter;
//! a per-agent adapter crate arrives only at multi-agent.

use contracts::{CorrelationId, HookEvent, HookKind, SessionId};
use serde_json::Value;

/// Translates an agent's raw hook bytes into a canonical [`HookEvent`].
///
/// Pure and dyn-safe so the gate holds it as `Arc<dyn AgentAdapter>` and stays
/// agent-agnostic.
pub trait AgentAdapter: Send + Sync {
    /// Parse raw hook bytes into a canonical event, or return why parsing failed.
    fn parse_hook(&self, body: &[u8]) -> Result<HookEvent, ParseError>;
}

/// Why a raw hook payload could not be turned into a [`HookEvent`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    /// The payload was not valid JSON, or contained an unrecognized event name.
    #[error("invalid hook json: {0}")]
    InvalidJson(String),
    /// A required field was absent from the payload.
    #[error("missing field: {0}")]
    MissingField(String),
}

/// The v1 agent adapter: maps the agent's snake-case hook envelope (discriminated
/// by `hook_event_name`) onto the canonical event kinds.
pub struct V1Adapter;

impl AgentAdapter for V1Adapter {
    fn parse_hook(&self, body: &[u8]) -> Result<HookEvent, ParseError> {
        let raw: Value =
            serde_json::from_slice(body).map_err(|e| ParseError::InvalidJson(e.to_string()))?;

        let kind = match field_str(&raw, "hook_event_name")? {
            "SessionStart" => HookKind::SessionStart {
                cwd: opt_str(&raw, "cwd"),
                client: opt_str(&raw, "client"),
                cli_version: opt_str(&raw, "cli_version"),
            },
            "UserPromptSubmit" => HookKind::UserPromptSubmit {
                content: field_str(&raw, "prompt")?.to_string(),
                turn_index: opt_i64(&raw, "turn_index"),
            },
            "PostToolUse" => HookKind::PostToolUse {
                tool_name: field_str(&raw, "tool_name")?.to_string(),
                tool_input: field_value(&raw, "tool_input")?,
                tool_response: field_str(&raw, "tool_response")?.to_string(),
                turn_index: field_i64(&raw, "turn_index")?,
            },
            "PermissionRequest" => HookKind::PermissionRequest {
                tool_name: opt_str(&raw, "tool_name"),
                request: field_value(&raw, "request")?,
            },
            "Stop" => HookKind::Stop {
                turn_index: opt_i64(&raw, "turn_index"),
            },
            "SessionEnd" => HookKind::SessionEnd {
                reason: opt_str(&raw, "reason"),
            },
            other => {
                return Err(ParseError::InvalidJson(format!(
                    "unknown hook_event_name: {other}"
                )))
            }
        };

        Ok(HookEvent {
            session_id: SessionId(field_str(&raw, "session_id")?.to_string()),
            // A placeholder the router-bound Normalize overwrites from ctx; the
            // agent never knows the gate's correlation id.
            correlation_id: CorrelationId(String::new()),
            // D11: the adapter is clock-free. It carries a timestamp only when the
            // agent reported one; an absent timestamp leaves ts == 0 for the gate
            // to stamp at receive time.
            ts: opt_i64(&raw, "timestamp_ms").unwrap_or(0),
            kind,
        })
    }
}

fn field_str<'a>(raw: &'a Value, key: &str) -> Result<&'a str, ParseError> {
    raw.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::MissingField(key.to_string()))
}

fn field_i64(raw: &Value, key: &str) -> Result<i64, ParseError> {
    raw.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| ParseError::MissingField(key.to_string()))
}

fn field_value(raw: &Value, key: &str) -> Result<Value, ParseError> {
    raw.get(key)
        .cloned()
        .ok_or_else(|| ParseError::MissingField(key.to_string()))
}

fn opt_str(raw: &Value, key: &str) -> Option<String> {
    raw.get(key).and_then(Value::as_str).map(str::to_string)
}

fn opt_i64(raw: &Value, key: &str) -> Option<i64> {
    raw.get(key).and_then(Value::as_i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(raw: Value) -> Result<HookEvent, ParseError> {
        V1Adapter.parse_hook(raw.to_string().as_bytes())
    }

    #[test]
    fn parses_session_start() {
        let event = parse(json!({
            "hook_event_name": "SessionStart",
            "session_id": "s-1",
            "timestamp_ms": 1000,
            "cwd": "/work",
            "client": "cli",
            "cli_version": "9.9.9",
        }))
        .unwrap();

        assert_eq!(
            event,
            HookEvent {
                session_id: SessionId("s-1".into()),
                correlation_id: CorrelationId(String::new()),
                ts: 1000,
                kind: HookKind::SessionStart {
                    cwd: Some("/work".into()),
                    client: Some("cli".into()),
                    cli_version: Some("9.9.9".into()),
                },
            }
        );
    }

    #[test]
    fn parses_user_prompt_submit() {
        let event = parse(json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "s-2",
            "timestamp_ms": 2000,
            "prompt": "hello",
            "turn_index": 1,
        }))
        .unwrap();

        assert_eq!(
            event.kind,
            HookKind::UserPromptSubmit {
                content: "hello".into(),
                turn_index: Some(1),
            }
        );
    }

    #[test]
    fn parses_post_tool_use() {
        let event = parse(json!({
            "hook_event_name": "PostToolUse",
            "session_id": "s-3",
            "timestamp_ms": 3000,
            "tool_name": "Bash",
            "tool_input": { "command": "ls" },
            "tool_response": "out",
            "turn_index": 2,
        }))
        .unwrap();

        assert_eq!(
            event.kind,
            HookKind::PostToolUse {
                tool_name: "Bash".into(),
                tool_input: json!({ "command": "ls" }),
                tool_response: "out".into(),
                turn_index: 2,
            }
        );
    }

    #[test]
    fn parses_permission_request() {
        let event = parse(json!({
            "hook_event_name": "PermissionRequest",
            "session_id": "s-4",
            "timestamp_ms": 4000,
            "tool_name": "Write",
            "request": { "path": "/x" },
        }))
        .unwrap();

        assert_eq!(
            event.kind,
            HookKind::PermissionRequest {
                tool_name: Some("Write".into()),
                request: json!({ "path": "/x" }),
            }
        );
    }

    #[test]
    fn parses_stop() {
        let event = parse(json!({
            "hook_event_name": "Stop",
            "session_id": "s-5",
            "timestamp_ms": 5000,
            "turn_index": 4,
        }))
        .unwrap();

        assert_eq!(
            event.kind,
            HookKind::Stop {
                turn_index: Some(4)
            }
        );
    }

    #[test]
    fn parses_session_end() {
        let event = parse(json!({
            "hook_event_name": "SessionEnd",
            "session_id": "s-6",
            "timestamp_ms": 6000,
            "reason": "done",
        }))
        .unwrap();

        assert_eq!(
            event.kind,
            HookKind::SessionEnd {
                reason: Some("done".into()),
            }
        );
    }

    #[test]
    fn parses_a_hook_without_a_timestamp_and_leaves_ts_unset() {
        // D11: the adapter stays clock-free; an absent timestamp yields ts == 0
        // for the gate to stamp at receive time.
        let event = parse(json!({
            "hook_event_name": "Stop",
            "session_id": "s-8",
        }))
        .unwrap();

        assert_eq!(event.ts, 0);
    }

    #[test]
    fn rejects_invalid_json() {
        let err = V1Adapter.parse_hook(b"{not json").unwrap_err();
        assert!(matches!(err, ParseError::InvalidJson(_)));
    }

    #[test]
    fn rejects_a_missing_required_field() {
        let err = parse(json!({
            "hook_event_name": "PostToolUse",
            "session_id": "s-7",
            "timestamp_ms": 7000,
            "tool_input": {},
            "tool_response": "r",
            "turn_index": 1,
        }))
        .unwrap_err();

        assert_eq!(err, ParseError::MissingField("tool_name".into()));
    }
}
