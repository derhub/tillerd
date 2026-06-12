use contracts::{AgentStatus, ContentEvent, HookEvent, HookKind};

/// Maps a hook event to an agent status. Pure and total — never panics.
pub fn hook_to_status(event: &HookEvent) -> AgentStatus {
    match &event.kind {
        HookKind::SessionStart { .. } => AgentStatus::Idle,
        HookKind::UserPromptSubmit { .. } => AgentStatus::Working,
        HookKind::PostToolUse { .. } => AgentStatus::Working,
        HookKind::PermissionRequest { .. } => AgentStatus::WaitingInput,
        HookKind::Stop { .. } => AgentStatus::Idle,
        HookKind::SessionEnd { .. } => AgentStatus::Done,
    }
}

/// Maps a hook event to an optional content event.
/// Returns `Some` only for `PostToolUse`; all other variants return `None`.
pub fn hook_to_content(event: &HookEvent) -> Option<ContentEvent> {
    match &event.kind {
        HookKind::PostToolUse {
            tool_name,
            tool_input,
            ..
        } => Some(ContentEvent {
            kind: "tool_use".to_string(),
            tool_name: tool_name.clone(),
            tool_input: tool_input.clone(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::{CorrelationId, SessionId};
    use serde_json::json;

    fn event(kind: HookKind) -> HookEvent {
        HookEvent {
            session_id: SessionId("s1".into()),
            correlation_id: CorrelationId("c1".into()),
            ts: 0,
            kind,
        }
    }

    #[test]
    fn session_start_maps_to_idle() {
        let e = event(HookKind::SessionStart {
            cwd: None,
            client: None,
            cli_version: None,
        });
        assert_eq!(hook_to_status(&e), AgentStatus::Idle);
    }

    #[test]
    fn user_prompt_submit_maps_to_working() {
        let e = event(HookKind::UserPromptSubmit {
            content: "hello".into(),
            turn_index: None,
        });
        assert_eq!(hook_to_status(&e), AgentStatus::Working);
    }

    #[test]
    fn post_tool_use_maps_to_working() {
        let e = event(HookKind::PostToolUse {
            tool_name: "Bash".into(),
            tool_input: json!({}),
            tool_response: "ok".into(),
            turn_index: 0,
        });
        assert_eq!(hook_to_status(&e), AgentStatus::Working);
    }

    #[test]
    fn permission_request_maps_to_waiting_input() {
        let e = event(HookKind::PermissionRequest {
            tool_name: None,
            request: json!({}),
        });
        assert_eq!(hook_to_status(&e), AgentStatus::WaitingInput);
    }

    #[test]
    fn stop_maps_to_idle() {
        let e = event(HookKind::Stop { turn_index: None });
        assert_eq!(hook_to_status(&e), AgentStatus::Idle);
    }

    #[test]
    fn session_end_maps_to_done() {
        let e = event(HookKind::SessionEnd { reason: None });
        assert_eq!(hook_to_status(&e), AgentStatus::Done);
    }

    #[test]
    fn post_tool_use_produces_content_event() {
        let e = event(HookKind::PostToolUse {
            tool_name: "Read".into(),
            tool_input: json!({ "path": "/foo" }),
            tool_response: "contents".into(),
            turn_index: 1,
        });
        let content = hook_to_content(&e).expect("Some");
        assert_eq!(content.kind, "tool_use");
        assert_eq!(content.tool_name, "Read");
        assert_eq!(content.tool_input, json!({ "path": "/foo" }));
    }

    #[test]
    fn non_post_tool_use_variants_return_none() {
        let variants = [
            event(HookKind::SessionStart {
                cwd: None,
                client: None,
                cli_version: None,
            }),
            event(HookKind::UserPromptSubmit {
                content: "hi".into(),
                turn_index: None,
            }),
            event(HookKind::PermissionRequest {
                tool_name: None,
                request: json!({}),
            }),
            event(HookKind::Stop { turn_index: None }),
            event(HookKind::SessionEnd { reason: None }),
        ];
        for e in &variants {
            assert!(
                hook_to_content(e).is_none(),
                "expected None for {:?}",
                e.kind
            );
        }
    }
}
