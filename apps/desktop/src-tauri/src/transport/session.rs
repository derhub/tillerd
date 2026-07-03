use orchestrator::app::session::LaunchSpecView;
use orchestrator::app::session::{
    ApplyLaunchSpec, ArchiveSession, ArrangePanels, DiscardSession, DuplicateSession,
    GetLaunchSpec, GetPanelTree, GetSessionById, LaunchSession, ListAllSessions,
    ListSessionsByProject, MoveSession, NewSessionCmd, PinSession, RenameSession, ReorderSession,
    RestoreSession, SearchSessions, SessionView, StopSessionSurfaces, UnpinSession,
};
use tauri::State;
use uuid::Uuid;

use crate::transport::macros::{transport_command, transport_create, transport_query};
use crate::transport::Bus;

// Omitted project_id lists all sessions. Hand-written, not `transport_query!`: it
// dispatches between two queries (`ListSessionsByProject` / `ListAllSessions`) on the
// optional project id -- dispatch logic, not boilerplate the macro abstracts.
#[tauri::command]
#[specta::specta]
pub async fn session_list(
    project_id: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    bus: State<'_, Bus>,
) -> Result<Vec<SessionView>, String> {
    let listing = match project_id {
        Some(pid) => {
            bus.query(ListSessionsByProject {
                project_id: pid,
                limit,
                offset,
                after: None,
            })
            .await
        }
        None => {
            bus.query(ListAllSessions {
                limit,
                offset,
                after: None,
            })
            .await
        }
    }
    .map_err(|e| e.to_string())?;
    Ok(listing.items)
}

transport_create!(
    /// Create a session, then bring its launch spec to life via the non-fatal tail
    /// (a spec-less session is valid; a launch failure does not invalidate the create).
    session_create(
        project_id: Option<String>,
        title: Option<String>,
        title_source: Option<String>,
        template_id: Option<String>,
    ) -> SessionView {
        let id = Uuid::new_v4().to_string();
        execute: NewSessionCmd {
            id: id.clone(),
            project_id: project_id
                .or_else(|| Some(orchestrator::app::project::unfiled_project_id())),
            title_source: title_source.unwrap_or_default(),
            title,
            template_id,
        },
        read_back: GetSessionById { id },
        map: |s| s,
        missing: "session vanished after create",
        tail: |created, bus| {
            let _ = bus
                .execute(LaunchSession {
                    id: created.id.clone(),
                })
                .await;
        },
    }
);

transport_command!(session_rename(id: String, title: String) => RenameSession {
    id,
    title,
});

transport_command!(session_archive(id: String) => ArchiveSession { id });

// Hard-delete a session. `DiscardSession` folds archive-then-discard internally
// (archives an active idle session then purges).
transport_command!(session_delete(id: String) => DiscardSession { id });

transport_command!(session_reorder(id: String, sort_order: u32) => ReorderSession {
    id,
    sort_order,
});

// `id` (not `session_id`) so the IPC arg matches the SDK and the other session commands;
// the wire `layoutJson` maps to the core `panel_tree_json`.
transport_command!(session_layout_set(id: String, layout_json: String) => ArrangePanels {
    id,
    panel_tree_json: layout_json,
});

transport_query!(
    session_layout_get(id: String) -> Option<String>
        => GetPanelTree { id },
        |tree| tree
);

transport_query!(
    session_get(id: String) -> Option<SessionView>
        => GetSessionById { id },
        |session| session
);

transport_query!(
    session_list_all(limit: Option<u32>, offset: Option<u32>, after: Option<String>) -> Vec<SessionView>
        => ListAllSessions { limit, offset, after },
        |listing| listing.items
);

transport_query!(
    session_get_launch_spec(id: String) -> Option<LaunchSpecView>
        => GetLaunchSpec { id },
        |spec| spec
);

transport_query!(
    session_search(query: String) -> Vec<SessionView>
        => SearchSessions { query },
        |results| results
);

transport_command!(session_launch(id: String) => LaunchSession { id });

transport_command!(
    session_apply_launch_spec(id: String, spec_version: u32, spec_json: String)
        => ApplyLaunchSpec { id, spec_version, spec_json }
);

transport_command!(session_move(id: String, target_project_id: String) => MoveSession {
    id,
    target_project_id,
});

transport_command!(session_duplicate(id: String) => DuplicateSession { id });

transport_command!(session_pin(id: String) => PinSession { id });

transport_command!(session_unpin(id: String) => UnpinSession { id });

transport_command!(session_restore(id: String) => RestoreSession { id });

transport_command!(session_stop_surfaces(id: String) => StopSessionSurfaces { id });

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_keys(value: &serde_json::Value, expected: &[&str]) {
        let obj = value.as_object().expect("response serializes to an object");
        let mut got: Vec<&str> = obj.keys().map(String::as_str).collect();
        got.sort_unstable();
        let mut want = expected.to_vec();
        want.sort_unstable();
        assert_eq!(got, want, "response keys drifted from the SDK contract");
    }

    #[test]
    fn session_response_matches_sdk_session_shape() {
        let s = SessionView {
            id: "s".into(),
            project_id: "p".into(),
            title: "T".into(),
            title_source: "agentTitle".into(),
            created_at: "2026-01-01T00:00:00.000Z".into(),
            status: "active".into(),
        };
        assert_keys(
            &serde_json::to_value(s).unwrap(),
            &[
                "id",
                "projectId",
                "title",
                "titleSource",
                "createdAt",
                "status",
            ],
        );
    }

    #[test]
    fn launch_spec_response_matches_sdk_launch_spec_shape() {
        let raw = serde_json::json!({ "version": 1, "items": [] });
        assert_keys(&raw, &["version", "items"]);
    }
}
