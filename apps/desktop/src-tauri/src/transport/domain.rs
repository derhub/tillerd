//! Macro-generated tauri shims for the workspace/project/session/command domain. Each
//! mechanical operation is one `transport_command!`/`transport_query!`/`transport_create!`
//! listing over the pure `app/` CQS type; the macro owns the `#[tauri::command]` wrapper,
//! the bus dispatch, the `shared::Error` -> wire-string mapping, and (for queries) the
//! mapping through the curated wire DTO. The wire (command names, argument shapes,
//! response JSON, error strings) is byte-identical to the prior hand-written shims.
//!
//! A handful of shims stay hand-written below because they are not mechanical: the
//! list-diff creates that mint server-side (`project_create`, `session_create`,
//! `command_create`) — the transport cannot read the entity back by a known id and
//! `session_create` also fires `LaunchSession` — and so cannot use `transport_create!`.

use std::collections::HashMap;

use orchestrator::app::command::{
    DiscardCommand, GetCommandById, ListCommands, NewCommand as NewCommandCmd,
};
use orchestrator::app::project::{
    ArchiveProject, DiscardProject, ListProjectsByWorkspace, MoveProject, NewProjectCmd,
    RenameProject, ReorderProject,
};
use orchestrator::app::session::{ArchiveSession, DiscardSession};
use orchestrator::app::session::{
    ArrangePanels, GetPanelTree, LaunchSession, ListAllSessions, ListSessionsByProject,
    NewSessionCmd, RenameSession, ReorderSession,
};
use orchestrator::app::workspace::{
    DiscardWorkspace, GetWorkspaceById, ListWorkspaces, NewWorkspaceCmd, RenameWorkspace,
    ReorderWorkspace,
};
use orchestrator::entities::{
    Command, CommandId, CommandOrigin, LaunchTemplateId, NewProject, NewSession, Project,
    ProjectId, Session, SessionId, SourceKind, TitleSource, Workspace, WorkspaceId,
};
use orchestrator::shared::pagination::Page;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::transport::macros::{transport_command, transport_create, transport_query};
use crate::transport::Bus;

// ── serializable response types ───────────────────────────────────────────────

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub source_kind: String,
    pub root_path: Option<String>,
    pub workspace_id: String,
}

fn project_response(p: Project) -> ProjectResponse {
    ProjectResponse {
        id: p.id.as_str().to_string(),
        name: p.name,
        source_kind: p.source_kind.as_str().to_string(),
        root_path: p.root_path,
        workspace_id: p.workspace_id.as_str().to_string(),
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceResponse {
    pub id: String,
    pub name: String,
}

fn workspace_response(w: Workspace) -> WorkspaceResponse {
    WorkspaceResponse {
        id: w.id.as_str().to_string(),
        name: w.name,
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub title_source: String,
    pub created_at: String,
}

fn session_response(s: Session) -> SessionResponse {
    SessionResponse {
        id: s.id.as_str().to_string(),
        project_id: s.project_id.as_str().to_string(),
        title: s.title,
        title_source: s.title_source.as_str().to_string(),
        created_at: s.created_at,
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandResponse {
    pub id: String,
    pub name: String,
    pub origin: String,
    pub cli: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

fn command_response(c: Command) -> CommandResponse {
    CommandResponse {
        id: c.id.as_str().to_string(),
        name: c.name,
        origin: c.origin.as_str().to_string(),
        cli: c.cli,
        args: c.args,
        env: c.env,
    }
}

// ── project ───────────────────────────────────────────────────────────────────

transport_query!(
    project_list(workspace_id: Option<String>) -> Vec<ProjectResponse>
        => ListProjectsByWorkspace {
            workspace_id: workspace_id.map(WorkspaceId::new).unwrap_or_else(WorkspaceId::default_id),
            page: Page::All,
        },
        |listing| listing.items.into_iter().map(project_response).collect()
);

transport_command!(project_rename(id: String, name: String) => RenameProject {
    id: ProjectId::new(id),
    name,
});

transport_command!(project_archive(id: String) => ArchiveProject { id: ProjectId::new(id) });

// Hard-delete a project. `DiscardProject` folds archive-then-discard internally
// (archives an active project, cascading its sessions, then purges).
transport_command!(project_delete(id: String) => DiscardProject { id: ProjectId::new(id) });

transport_command!(project_reorder(id: String, sort_order: u32) => ReorderProject {
    id: ProjectId::new(id),
    sort_order,
});

transport_command!(project_move(id: String, workspace_id: String) => MoveProject {
    id: ProjectId::new(id),
    workspace_id: WorkspaceId::new(workspace_id),
});

/// Create a project (server-mints the id and infers the name; list-diff finds the new
/// row). Not a `transport_create!` because the transport never learns the minted id.
#[tauri::command]
pub async fn project_create(
    name: Option<String>,
    workspace_id: Option<String>,
    bus: State<'_, Bus>,
) -> Result<ProjectResponse, String> {
    let workspace = workspace_id
        .map(WorkspaceId::new)
        .unwrap_or_else(WorkspaceId::default_id);

    let before: Vec<String> = bus
        .query(ListProjectsByWorkspace {
            workspace_id: workspace.clone(),
            page: Page::All,
        })
        .await
        .map_err(|e| e.to_string())?
        .items
        .into_iter()
        .map(|p| p.id.as_str().to_string())
        .collect();

    bus.execute(NewProjectCmd {
        params: NewProject {
            source_kind: SourceKind::Blank,
            root_path: None,
            name,
            workspace_id: Some(workspace.clone()),
        },
    })
    .await
    .map_err(|e| e.to_string())?;

    let project = bus
        .query(ListProjectsByWorkspace {
            workspace_id: workspace,
            page: Page::All,
        })
        .await
        .map_err(|e| e.to_string())?
        .items
        .into_iter()
        .find(|p| !before.contains(&p.id.as_str().to_string()))
        .ok_or_else(|| "project vanished after create".to_string())?;
    Ok(project_response(project))
}

// ── workspace ───────────────────────────────────────────────────────────────────

transport_create!(
    workspace_create(name: String) -> WorkspaceResponse {
        let id = WorkspaceId::new(Uuid::new_v4().to_string());
        execute: NewWorkspaceCmd { id: id.clone(), name },
        read_back: GetWorkspaceById { id },
        map: |workspace| workspace_response(workspace),
        missing: "workspace vanished after create",
    }
);

transport_query!(
    workspace_list() -> Vec<WorkspaceResponse>
        => ListWorkspaces { page: Page::All },
        |listing| listing.items.into_iter().map(workspace_response).collect()
);

transport_command!(workspace_rename(id: String, name: String) => RenameWorkspace {
    id: WorkspaceId::new(id),
    name,
});

transport_command!(workspace_reorder(id: String, sort_order: u32) => ReorderWorkspace {
    id: WorkspaceId::new(id),
    sort_order,
});

transport_command!(workspace_delete(id: String) => DiscardWorkspace { id: WorkspaceId::new(id) });

// ── session ───────────────────────────────────────────────────────────────────

// `project_id` omitted => list EVERY session (the sidebar groups all sessions by
// project); given => that project only. Matches the pre-refactor wire.
#[tauri::command]
pub async fn session_list(
    project_id: Option<String>,
    bus: State<'_, Bus>,
) -> Result<Vec<SessionResponse>, String> {
    let listing = match project_id {
        Some(pid) => {
            bus.query(ListSessionsByProject {
                project_id: ProjectId::new(pid),
                page: Page::All,
            })
            .await
        }
        None => bus.query(ListAllSessions { page: Page::All }).await,
    }
    .map_err(|e| e.to_string())?;
    Ok(listing.items.into_iter().map(session_response).collect())
}

fn parse_title_source(s: Option<&str>) -> TitleSource {
    match s {
        Some("branch") => TitleSource::Branch,
        Some("both") => TitleSource::Both,
        Some("custom") => TitleSource::Custom,
        _ => TitleSource::AgentTitle,
    }
}

/// Create a session (server-mints the id; list-diff finds the new row) then brings its
/// launch spec to life. Not a `transport_create!`: the transport never learns the minted
/// id and the create chains a non-fatal `LaunchSession`.
#[tauri::command]
pub async fn session_create(
    project_id: Option<String>,
    title: Option<String>,
    title_source: Option<String>,
    template_id: Option<String>,
    bus: State<'_, Bus>,
) -> Result<SessionResponse, String> {
    let pid = project_id
        .map(ProjectId::new)
        .unwrap_or_else(ProjectId::unfiled);
    let draft = NewSession {
        project_id: Some(pid.clone()),
        title_source: parse_title_source(title_source.as_deref()),
        title,
        template_id: template_id.map(LaunchTemplateId::from_string),
    };

    let before: Vec<String> = bus
        .query(ListSessionsByProject {
            project_id: pid.clone(),
            page: Page::All,
        })
        .await
        .map_err(|e| e.to_string())?
        .items
        .into_iter()
        .map(|s| s.id.as_str().to_string())
        .collect();

    bus.execute(NewSessionCmd(draft))
        .await
        .map_err(|e| e.to_string())?;

    let created = bus
        .query(ListSessionsByProject {
            project_id: pid,
            page: Page::All,
        })
        .await
        .map_err(|e| e.to_string())?
        .items
        .into_iter()
        .find(|s| !before.contains(&s.id.as_str().to_string()))
        .ok_or_else(|| "session vanished after create".to_string())?;

    // Bring the session's launch spec to life (a session with no spec launches
    // nothing). Non-fatal: a runtime failure does not invalidate the created session.
    let _ = bus
        .execute(LaunchSession {
            id: created.id.clone(),
        })
        .await;

    Ok(session_response(created))
}

transport_command!(session_rename(id: String, title: String) => RenameSession {
    id: SessionId::from_string(id),
    title,
});

transport_command!(session_archive(id: String) => ArchiveSession { id: SessionId::from_string(id) });

// Hard-delete a session. `DiscardSession` folds archive-then-discard internally
// (archives an active idle session then purges).
transport_command!(session_delete(id: String) => DiscardSession { id: SessionId::from_string(id) });

transport_command!(session_reorder(id: String, sort_order: u32) => ReorderSession {
    id: SessionId::from_string(id),
    sort_order,
});

// `id` (not `session_id`) so the IPC arg matches the SDK and the other session commands;
// the wire `layoutJson` maps to the core `panel_tree_json`.
transport_command!(session_layout_set(id: String, layout_json: String) => ArrangePanels {
    id: SessionId::from_string(id),
    panel_tree_json: layout_json,
});

transport_query!(
    session_layout_get(id: String) -> Option<String>
        => GetPanelTree { id: SessionId::from_string(id) },
        |tree| tree
);

// ── command library ───────────────────────────────────────────────────────────

transport_query!(
    command_get(id: String) -> Option<CommandResponse>
        => GetCommandById { id: CommandId::from_string(id) },
        |cmd| cmd.map(command_response)
);

transport_command!(command_delete(id: String) => DiscardCommand { id: CommandId::from_string(id) });

transport_query!(
    command_list() -> Vec<CommandResponse>
        => ListCommands { origin: None, page: Page::All },
        |listing| listing.items.into_iter().map(command_response).collect()
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

/// Create a library command (server-mints the id; list-diff finds the new row). Not a
/// `transport_create!`: the transport never learns the minted id.
#[tauri::command]
pub async fn command_create(
    req: CreateCommandRequest,
    bus: State<'_, Bus>,
) -> Result<CommandResponse, String> {
    let existing: Vec<String> = bus
        .query(ListCommands {
            origin: Some(CommandOrigin::Custom),
            page: Page::All,
        })
        .await
        .map_err(|e| e.to_string())?
        .items
        .into_iter()
        .map(|c| c.id.as_str().to_string())
        .collect();

    bus.execute(NewCommandCmd {
        name: req.name.clone(),
        cli: req.cli,
        args: req.args,
        env: req.env,
    })
    .await
    .map_err(|e| e.to_string())?;

    let created = bus
        .query(ListCommands {
            origin: Some(CommandOrigin::Custom),
            page: Page::All,
        })
        .await
        .map_err(|e| e.to_string())?
        .items
        .into_iter()
        .find(|c| !existing.contains(&c.id.as_str().to_string()))
        .ok_or_else(|| "command vanished after create".to_string())?;

    Ok(command_response(created))
}

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
        let p = ProjectResponse {
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
        let s = SessionResponse {
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
        let c = CommandResponse {
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
        let w = WorkspaceResponse {
            id: "w".into(),
            name: "W".into(),
        };
        assert_keys(&serde_json::to_value(w).unwrap(), &["id", "name"]);
    }

    #[test]
    fn parse_title_source_maps_wire_strings() {
        assert!(matches!(
            parse_title_source(Some("custom")),
            TitleSource::Custom
        ));
        assert!(matches!(
            parse_title_source(Some("branch")),
            TitleSource::Branch
        ));
        assert!(matches!(parse_title_source(None), TitleSource::AgentTitle));
    }
}
