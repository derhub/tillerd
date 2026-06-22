//! Macro-generated tauri shims for the workspace/project/session/command domain. Each
//! mechanical operation is one `transport_command!`/`transport_query!`/`transport_create!`
//! listing over the pure `app/` CQS type; the macro owns the `#[tauri::command]` wrapper,
//! the bus dispatch, the `shared::Error` -> wire-string mapping, and (for queries) the
//! mapping through the curated wire DTO. The wire (command names, argument shapes,
//! response JSON, error strings) is byte-identical to the prior hand-written shims.
//!
//! `session_create` stays hand-written because it chains a non-fatal `LaunchSession` after
//! the create -- a tail the `transport_create!` macro does not model.

use std::collections::HashMap;

use orchestrator::app::command::{
    CommandId, CommandView, DiscardCommand, GetCommandById, ListCommands,
    NewCommand as NewCommandCmd,
};
use orchestrator::app::project::{
    ArchiveProject, DiscardProject, GetProjectById, ListProjectsByWorkspace, MoveProject,
    NewProjectCmd, ProjectId, ProjectView, RenameProject, ReorderProject,
};
use orchestrator::app::session::{ArchiveSession, DiscardSession};
use orchestrator::app::session::{
    ArrangePanels, GetPanelTree, GetSessionById, LaunchSession, ListAllSessions,
    ListSessionsByProject, NewSessionCmd, RenameSession, ReorderSession, SessionId, SessionView,
};
use orchestrator::app::workspace::{
    DiscardWorkspace, GetWorkspaceById, ListWorkspaces, NewWorkspaceCmd, RenameWorkspace,
    ReorderWorkspace, WorkspaceView,
};
use serde::Deserialize;
use tauri::State;
use uuid::Uuid;

use crate::transport::macros::{transport_command, transport_create, transport_query};
use crate::transport::Bus;

// -- project -------------------------------------------------------------------

transport_query!(
    project_list(workspace_id: Option<String>) -> Vec<ProjectView>
        => ListProjectsByWorkspace {
            workspace_id: workspace_id.unwrap_or_else(orchestrator::app::workspace::default_workspace_id),
            limit: None,
            offset: None,
            after: None,
        },
        |listing| listing.items
);

transport_command!(project_rename(id: String, name: String) => RenameProject {
    id,
    name,
});

transport_command!(project_archive(id: String) => ArchiveProject { id });

// Hard-delete a project. `DiscardProject` folds archive-then-discard internally
// (archives an active project, cascading its sessions, then purges).
transport_command!(project_delete(id: String) => DiscardProject { id });

transport_command!(project_reorder(id: String, sort_order: u32) => ReorderProject {
    id,
    sort_order,
});

transport_command!(project_move(id: String, workspace_id: String) => MoveProject {
    id,
    workspace_id,
});

transport_create!(
    project_create(name: Option<String>, workspace_id: Option<String>) -> ProjectView {
        let id = ProjectId::new(Uuid::new_v4().to_string());
        execute: NewProjectCmd {
            id: id.clone(),
            source_kind: "blank".to_string(),
            root_path: None,
            name,
            workspace_id: workspace_id
                .or_else(|| Some(orchestrator::app::workspace::default_workspace_id())),
        },
        read_back: GetProjectById { id: id.as_str().to_string() },
        map: |project| project,
        missing: "project vanished after create",
    }
);

// -- workspace -------------------------------------------------------------------

transport_create!(
    workspace_create(name: String) -> WorkspaceView {
        let id = Uuid::new_v4().to_string();
        execute: NewWorkspaceCmd { id: id.clone(), name },
        read_back: GetWorkspaceById { id },
        map: |workspace| workspace,
        missing: "workspace vanished after create",
    }
);

transport_query!(
    workspace_list() -> Vec<WorkspaceView>
        => ListWorkspaces { limit: None, offset: None, after: None },
        |listing| listing.items
);

transport_command!(workspace_rename(id: String, name: String) => RenameWorkspace {
    id,
    name,
});

transport_command!(workspace_reorder(id: String, sort_order: u32) => ReorderWorkspace {
    id,
    sort_order,
});

transport_command!(workspace_delete(id: String) => DiscardWorkspace { id });

// -- session -------------------------------------------------------------------

// `project_id` omitted => list EVERY session (the sidebar groups all sessions by
// project); given => that project only. Matches the pre-refactor wire.
#[tauri::command]
pub async fn session_list(
    project_id: Option<String>,
    bus: State<'_, Bus>,
) -> Result<Vec<SessionView>, String> {
    let listing = match project_id {
        Some(pid) => {
            bus.query(ListSessionsByProject {
                project_id: pid,
                limit: None,
                offset: None,
                after: None,
            })
            .await
        }
        None => {
            bus.query(ListAllSessions {
                limit: None,
                offset: None,
                after: None,
            })
            .await
        }
    }
    .map_err(|e| e.to_string())?;
    Ok(listing.items)
}

/// Create a session, bring its launch spec to life. The `LaunchSession` tail is
/// non-fatal (a spec-less session is valid), so this stays hand-written rather than
/// using `transport_create!`.
#[tauri::command]
pub async fn session_create(
    project_id: Option<String>,
    title: Option<String>,
    title_source: Option<String>,
    template_id: Option<String>,
    bus: State<'_, Bus>,
) -> Result<SessionView, String> {
    let id = SessionId::mint();

    bus.execute(NewSessionCmd {
        id: id.clone(),
        project_id: project_id.or_else(|| Some(orchestrator::app::project::unfiled_project_id())),
        title_source: title_source.unwrap_or_default(),
        title,
        template_id,
    })
    .await
    .map_err(|e| e.to_string())?;

    let created = bus
        .query(GetSessionById {
            id: id.as_str().to_string(),
        })
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "session vanished after create".to_string())?;

    // Bring the session's launch spec to life (a session with no spec launches
    // nothing). Non-fatal: a runtime failure does not invalidate the created session.
    let _ = bus
        .execute(LaunchSession {
            id: created.id.clone(),
        })
        .await;

    Ok(created)
}

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

// -- command library -----------------------------------------------------------

transport_query!(
    command_get(id: String) -> Option<CommandView>
        => GetCommandById { id },
        |cmd| cmd
);

transport_command!(command_delete(id: String) => DiscardCommand { id });

transport_query!(
    command_list() -> Vec<CommandView>
        => ListCommands { origin: None, limit: None, offset: None, after: None },
        |listing| listing.items
);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCommandRequest {
    pub name: String,
    pub cli: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

transport_create!(
    command_create(req: CreateCommandRequest) -> CommandView {
        let id = CommandId::mint();
        execute: NewCommandCmd {
            id: id.clone(),
            name: req.name,
            cli: req.cli,
            args: req.args,
            env: req.env,
        },
        read_back: GetCommandById { id: id.as_str().to_string() },
        map: |cmd| cmd,
        missing: "command vanished after create",
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    /// Assert a serialized response carries exactly the camelCase keys the SDK type declares, so
    /// shim response shapes can't silently drift from the `@tillerd/sdk` contract.
    fn assert_keys(value: &serde_json::Value, expected: &[&str]) {
        let obj = value.as_object().expect("response serializes to an object");
        let mut got: Vec<&str> = obj.keys().map(String::as_str).collect();
        got.sort_unstable();
        let mut want = expected.to_vec();
        want.sort_unstable();
        assert_eq!(got, want, "response keys drifted from the SDK contract");
    }

    #[test]
    fn project_response_matches_sdk_project_shape() {
        let p = ProjectView {
            id: "p".into(),
            name: "P".into(),
            source_kind: "blank".into(),
            root_path: None,
            workspace_id: "w".into(),
        };
        assert_keys(
            &serde_json::to_value(p).unwrap(),
            &["id", "name", "sourceKind", "rootPath", "workspaceId"],
        );
    }

    #[test]
    fn session_response_matches_sdk_session_shape() {
        let s = SessionView {
            id: "s".into(),
            project_id: "p".into(),
            title: "T".into(),
            title_source: "agentTitle".into(),
            created_at: "2026-01-01T00:00:00.000Z".into(),
        };
        assert_keys(
            &serde_json::to_value(s).unwrap(),
            &["id", "projectId", "title", "titleSource", "createdAt"],
        );
    }

    #[test]
    fn command_response_matches_sdk_command_shape() {
        let c = CommandView {
            id: "c".into(),
            name: "c".into(),
            origin: "custom".into(),
            cli: "/c".into(),
            args: vec![],
            env: Default::default(),
        };
        assert_keys(
            &serde_json::to_value(c).unwrap(),
            &["id", "name", "origin", "cli", "args", "env"],
        );
    }

    #[test]
    fn workspace_response_matches_sdk_workspace_shape() {
        let w = WorkspaceView {
            id: "w".into(),
            name: "W".into(),
        };
        assert_keys(&serde_json::to_value(w).unwrap(), &["id", "name"]);
    }
}
