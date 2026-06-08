//! Routes a normalized [`HookEvent`] to memorya's existing capture API. The
//! dispatcher only routes; redaction, skip-listing, titling, and duplicate
//! suppression already live in `capture_prompt`/`capture_tool`/`ensure_session`.

use std::sync::{Arc, Mutex};

use contracts::{HookEvent, HookKind};

use crate::Engram;

/// Captures hook events into a shared memorya. The single `rusqlite` connection
/// is `!Sync`, so the memorya is shared behind a mutex.
pub struct HookCapturer {
    memorya: Arc<Mutex<Engram>>,
}

impl HookCapturer {
    /// A capturer over a shared memorya.
    pub fn new(memorya: Arc<Mutex<Engram>>) -> Self {
        Self { memorya }
    }

    /// Route one event to the matching capture path. Returns the new chunk id
    /// when a chunk was stored.
    pub fn dispatch(&self, event: &HookEvent) -> anyhow::Result<Option<i64>> {
        let memorya = self.memorya.lock().expect("memorya mutex poisoned");
        dispatch(&memorya, event)
    }
}

/// Route `event` into `memorya`. A chunk references its session, so capture paths
/// ensure the session row before any insert (foreign key).
fn dispatch(memorya: &Engram, event: &HookEvent) -> anyhow::Result<Option<i64>> {
    let session_id = event.session_id.0.as_str();
    let ts = event.ts;
    match &event.kind {
        HookKind::SessionStart { cwd, client, .. } => {
            memorya.ensure_session(
                session_id,
                client.as_deref().unwrap_or("unknown"),
                cwd.as_deref(),
                ts,
            )?;
            Ok(None)
        }
        HookKind::UserPromptSubmit {
            content,
            turn_index,
        } => {
            // An empty prompt carries no recall value.
            if content.is_empty() {
                return Ok(None);
            }
            memorya.ensure_session(session_id, "unknown", None, ts)?;
            memorya.capture_prompt(Some(session_id), content, *turn_index, ts)
        }
        HookKind::PostToolUse {
            tool_name,
            tool_input,
            tool_response,
            turn_index,
        } => {
            memorya.ensure_session(session_id, "unknown", None, ts)?;
            memorya.capture_tool(
                session_id,
                tool_name,
                tool_input,
                tool_response,
                *turn_index,
                ts,
            )
        }
        // PermissionRequest, Stop, SessionEnd, and any future kind are not
        // captured.
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::{CorrelationId, SessionId};
    use serde_json::json;

    fn capturer() -> (tempfile::TempDir, Arc<Mutex<Engram>>, HookCapturer) {
        let dir = tempfile::tempdir().unwrap();
        let memorya = Arc::new(Mutex::new(
            Engram::open(dir.path().join("memorya.db")).unwrap(),
        ));
        let cap = HookCapturer::new(memorya.clone());
        (dir, memorya, cap)
    }

    fn event(kind: HookKind) -> HookEvent {
        HookEvent {
            session_id: SessionId("s1".into()),
            correlation_id: CorrelationId("c1".into()),
            ts: 5,
            kind,
        }
    }

    fn chunk_count(memorya: &Arc<Mutex<Engram>>) -> i64 {
        memorya.lock().unwrap().active_chunk_count().unwrap()
    }

    #[test]
    fn session_start_ensures_session_row_with_client_and_cwd() {
        let (_d, memorya, cap) = capturer();
        cap.dispatch(&event(HookKind::SessionStart {
            cwd: Some("/proj".into()),
            client: Some("agent-cli".into()),
            cli_version: None,
        }))
        .unwrap();

        let guard = memorya.lock().unwrap();
        let (ide, cwd): (String, String) = guard
            .store()
            .conn()
            .query_row("SELECT ide, cwd FROM sessions WHERE id='s1'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!((ide.as_str(), cwd.as_str()), ("agent-cli", "/proj"));
    }

    #[test]
    fn user_prompt_submit_captures_prompt_chunk() {
        let (_d, memorya, cap) = capturer();
        let id = cap
            .dispatch(&event(HookKind::UserPromptSubmit {
                content: "hello world".into(),
                turn_index: Some(0),
            }))
            .unwrap();
        assert!(id.is_some());
        assert_eq!(chunk_count(&memorya), 1);
    }

    #[test]
    fn user_prompt_submit_with_empty_content_creates_no_chunk() {
        let (_d, memorya, cap) = capturer();
        let id = cap
            .dispatch(&event(HookKind::UserPromptSubmit {
                content: String::new(),
                turn_index: Some(0),
            }))
            .unwrap();
        assert!(id.is_none());
        assert_eq!(chunk_count(&memorya), 0);
    }

    #[test]
    fn post_tool_use_captures_tool_chunk_with_auto_title() {
        let (_d, memorya, cap) = capturer();
        cap.dispatch(&event(HookKind::PostToolUse {
            tool_name: "Read".into(),
            tool_input: json!({ "file_path": "src/x.rs" }),
            tool_response: "...".into(),
            turn_index: 2,
        }))
        .unwrap();

        let chunks = memorya.lock().unwrap().recent_chunks(10).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].1.as_deref(), Some("Read src/x.rs"));
    }

    #[test]
    fn post_tool_use_skips_low_value_tool_on_skip_list() {
        let (_d, memorya, cap) = capturer();
        let id = cap
            .dispatch(&event(HookKind::PostToolUse {
                tool_name: "TodoWrite".into(),
                tool_input: json!({}),
                tool_response: "x".into(),
                turn_index: 1,
            }))
            .unwrap();
        assert!(id.is_none());
        assert_eq!(chunk_count(&memorya), 0);
    }

    #[test]
    fn permission_request_is_skipped() {
        let (_d, memorya, cap) = capturer();
        let id = cap
            .dispatch(&event(HookKind::PermissionRequest {
                tool_name: Some("Bash".into()),
                request: json!({ "command": "ls" }),
            }))
            .unwrap();
        assert!(id.is_none());
        assert_eq!(chunk_count(&memorya), 0);
    }

    #[test]
    fn stop_is_skipped() {
        let (_d, memorya, cap) = capturer();
        let id = cap
            .dispatch(&event(HookKind::Stop {
                turn_index: Some(0),
            }))
            .unwrap();
        assert!(id.is_none());
        assert_eq!(chunk_count(&memorya), 0);
    }

    #[test]
    fn session_end_is_skipped() {
        let (_d, memorya, cap) = capturer();
        let id = cap
            .dispatch(&event(HookKind::SessionEnd {
                reason: Some("quit".into()),
            }))
            .unwrap();
        assert!(id.is_none());
        assert_eq!(chunk_count(&memorya), 0);
    }

    #[test]
    fn ensures_session_before_chunk_insert_satisfying_foreign_key() {
        let (_d, memorya, cap) = capturer();
        let id = cap
            .dispatch(&event(HookKind::PostToolUse {
                tool_name: "Read".into(),
                tool_input: json!({ "file_path": "src/a.rs" }),
                tool_response: "ok".into(),
                turn_index: 0,
            }))
            .unwrap();
        assert!(
            id.is_some(),
            "the chunk persists even without a prior session_start"
        );
        assert_eq!(chunk_count(&memorya), 1);
    }

    #[test]
    fn duplicate_hook_fire_is_idempotent() {
        let (_d, memorya, cap) = capturer();
        let prompt = event(HookKind::UserPromptSubmit {
            content: "same prompt".into(),
            turn_index: Some(0),
        });

        let first = cap.dispatch(&prompt).unwrap();
        let second = cap.dispatch(&prompt).unwrap();

        assert!(first.is_some());
        assert!(second.is_none(), "a duplicate fire is suppressed");
        assert_eq!(chunk_count(&memorya), 1);
    }
}
