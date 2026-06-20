use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
use orchestrator::app::create_session;
use orchestrator::app::open_session;
use orchestrator::entities::{
    Command, CommandOrigin, LaunchTemplateId, NewCommand, NewProject, NewSession, NewWorkspace,
    Project, ProjectId, Session, SessionId, SourceKind, TitleSource, Workspace, WorkspaceId,
};
#[cfg(test)]
use orchestrator::store::LaunchTemplates;
use orchestrator::store::{
    Commands, ProjectFilter, Projects, SessionFilter, Sessions, Storage, Workspaces,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::orchestrator_host::OrchestratorState;
use crate::surface_host::SurfaceState;

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

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WorkspaceError {
    NotFound { message: String },
    UnfiledGuard { message: String },
    Internal { message: String },
}

pub fn map_store_err(e: orchestrator::OrchestratorError) -> WorkspaceError {
    use orchestrator::OrchestratorError::*;
    match e {
        ProjectNotFound(m) | SessionNotFound(m) | CommandNotFound(m) | WorkspaceNotFound(m) => {
            WorkspaceError::NotFound { message: m }
        }
        ProjectIsUnfiled => WorkspaceError::UnfiledGuard {
            message: "the Unfiled project cannot be modified".to_string(),
        },
        WorkspaceIsDefault => WorkspaceError::UnfiledGuard {
            message: "the Default workspace cannot be deleted".to_string(),
        },
        other => WorkspaceError::Internal {
            message: other.to_string(),
        },
    }
}

fn storage_or_err(state: &OrchestratorState) -> Result<Arc<Storage>, WorkspaceError> {
    state.storage().ok_or_else(|| WorkspaceError::Internal {
        message: "orchestrator not ready".to_string(),
    })
}

// ── project ───────────────────────────────────────────────────────────────────

pub async fn do_project_create(
    projects: &Projects,
    name: Option<String>,
    workspace_id: Option<String>,
) -> Result<ProjectResponse, WorkspaceError> {
    let project = projects
        .create(NewProject {
            source_kind: SourceKind::Blank,
            root_path: None,
            name,
            workspace_id: workspace_id.map(WorkspaceId::new),
        })
        .await
        .map_err(map_store_err)?;
    Ok(project_response(project))
}

#[tauri::command]
pub async fn project_create(
    name: Option<String>,
    workspace_id: Option<String>,
    state: State<'_, OrchestratorState>,
) -> Result<ProjectResponse, String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_project_create(&storage.projects, name, workspace_id)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_project_list(
    projects: &Projects,
    workspace_id: Option<String>,
) -> Result<Vec<ProjectResponse>, WorkspaceError> {
    let filter = ProjectFilter {
        workspace: workspace_id.map(WorkspaceId::new),
    };
    let projects = projects.list(&filter).await.map_err(map_store_err)?;
    Ok(projects.into_iter().map(project_response).collect())
}

#[tauri::command]
pub async fn project_list(
    workspace_id: Option<String>,
    state: State<'_, OrchestratorState>,
) -> Result<Vec<ProjectResponse>, String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_project_list(&storage.projects, workspace_id)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_project_rename(
    projects: &Projects,
    id: String,
    name: String,
) -> Result<(), WorkspaceError> {
    projects
        .rename(ProjectId::new(id), name)
        .await
        .map_err(map_store_err)
}

#[tauri::command]
pub async fn project_rename(
    id: String,
    name: String,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_project_rename(&storage.projects, id, name)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_project_archive(projects: &Projects, id: String) -> Result<(), WorkspaceError> {
    projects
        .archive(ProjectId::new(id))
        .await
        .map_err(map_store_err)
}

#[tauri::command]
pub async fn project_archive(
    id: String,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_project_archive(&storage.projects, id)
        .await
        .map_err(|e| format!("{e:?}"))
}

/// Hard-delete a project. The store's `hard_delete` requires the project be archived first,
/// so a live project is archived (cascading its sessions/surfaces to soft-deleted) then purged in
/// one host call. An already-archived project skips straight to the purge.
pub async fn do_project_delete(projects: &Projects, id: String) -> Result<(), WorkspaceError> {
    let pid = ProjectId::new(id);
    match projects.archive(pid.clone()).await {
        Ok(()) => {}
        // Already archived (or absent): the purge below reports the definitive outcome.
        Err(orchestrator::OrchestratorError::ProjectNotFound(_)) => {}
        Err(e) => return Err(map_store_err(e)),
    }
    projects.hard_delete(pid).await.map_err(map_store_err)
}

#[tauri::command]
pub async fn project_delete(id: String, state: State<'_, OrchestratorState>) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_project_delete(&storage.projects, id)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_project_reorder(
    projects: &Projects,
    id: String,
    sort_order: u32,
) -> Result<(), WorkspaceError> {
    projects
        .reorder(ProjectId::new(id), sort_order)
        .await
        .map_err(map_store_err)
}

#[tauri::command]
pub async fn project_reorder(
    id: String,
    sort_order: u32,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_project_reorder(&storage.projects, id, sort_order)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_project_move(
    projects: &Projects,
    id: String,
    workspace_id: String,
) -> Result<(), WorkspaceError> {
    projects
        .move_to(ProjectId::new(id), WorkspaceId::new(workspace_id))
        .await
        .map_err(map_store_err)
}

#[tauri::command]
pub async fn project_move(
    id: String,
    workspace_id: String,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_project_move(&storage.projects, id, workspace_id)
        .await
        .map_err(|e| format!("{e:?}"))
}

// ── workspace ───────────────────────────────────────────────────────────────────

pub async fn do_workspace_create(
    workspaces: &Workspaces,
    name: String,
) -> Result<WorkspaceResponse, WorkspaceError> {
    let workspace = workspaces
        .create(NewWorkspace { name })
        .await
        .map_err(map_store_err)?;
    Ok(workspace_response(workspace))
}

#[tauri::command]
pub async fn workspace_create(
    name: String,
    state: State<'_, OrchestratorState>,
) -> Result<WorkspaceResponse, String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_workspace_create(&storage.workspaces, name)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_workspace_list(
    workspaces: &Workspaces,
) -> Result<Vec<WorkspaceResponse>, WorkspaceError> {
    let workspaces = workspaces.list().await.map_err(map_store_err)?;
    Ok(workspaces.into_iter().map(workspace_response).collect())
}

#[tauri::command]
pub async fn workspace_list(
    state: State<'_, OrchestratorState>,
) -> Result<Vec<WorkspaceResponse>, String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_workspace_list(&storage.workspaces)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_workspace_rename(
    workspaces: &Workspaces,
    id: String,
    name: String,
) -> Result<(), WorkspaceError> {
    workspaces
        .rename(WorkspaceId::new(id), name)
        .await
        .map_err(map_store_err)
}

#[tauri::command]
pub async fn workspace_rename(
    id: String,
    name: String,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_workspace_rename(&storage.workspaces, id, name)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_workspace_reorder(
    workspaces: &Workspaces,
    id: String,
    sort_order: u32,
) -> Result<(), WorkspaceError> {
    workspaces
        .reorder(WorkspaceId::new(id), sort_order)
        .await
        .map_err(map_store_err)
}

#[tauri::command]
pub async fn workspace_reorder(
    id: String,
    sort_order: u32,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_workspace_reorder(&storage.workspaces, id, sort_order)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_workspace_delete(
    workspaces: &Workspaces,
    id: String,
) -> Result<(), WorkspaceError> {
    workspaces
        .delete(WorkspaceId::new(id))
        .await
        .map_err(map_store_err)
}

#[tauri::command]
pub async fn workspace_delete(
    id: String,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_workspace_delete(&storage.workspaces, id)
        .await
        .map_err(|e| format!("{e:?}"))
}

// ── session ───────────────────────────────────────────────────────────────────

pub async fn do_session_list(
    sessions: &Sessions,
    project_id: Option<String>,
) -> Result<Vec<SessionResponse>, WorkspaceError> {
    let filter = SessionFilter {
        project: project_id.map(ProjectId::new),
    };
    let sessions = sessions.list(&filter).await.map_err(map_store_err)?;
    Ok(sessions.into_iter().map(session_response).collect())
}

#[tauri::command]
pub async fn session_list(
    project_id: Option<String>,
    state: State<'_, OrchestratorState>,
) -> Result<Vec<SessionResponse>, String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_session_list(&storage.sessions, project_id)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_session_rename(
    sessions: &Sessions,
    id: String,
    title: String,
) -> Result<(), WorkspaceError> {
    sessions
        .rename(SessionId::from_string(id), title)
        .await
        .map_err(map_store_err)
}

#[tauri::command]
pub async fn session_rename(
    id: String,
    title: String,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_session_rename(&storage.sessions, id, title)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_session_archive(sessions: &Sessions, id: String) -> Result<(), WorkspaceError> {
    sessions
        .archive(SessionId::from_string(id))
        .await
        .map_err(map_store_err)
}

#[tauri::command]
pub async fn session_archive(
    id: String,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_session_archive(&storage.sessions, id)
        .await
        .map_err(|e| format!("{e:?}"))
}

/// Hard-delete a session. Mirrors `do_project_delete`: archive first (the store's
/// `hard_delete` requires an archived row) then purge.
pub async fn do_session_delete(sessions: &Sessions, id: String) -> Result<(), WorkspaceError> {
    let sid = SessionId::from_string(id);
    match sessions.archive(sid.clone()).await {
        Ok(()) => {}
        Err(orchestrator::OrchestratorError::SessionNotFound(_)) => {}
        Err(e) => return Err(map_store_err(e)),
    }
    sessions.hard_delete(sid).await.map_err(map_store_err)
}

#[tauri::command]
pub async fn session_delete(id: String, state: State<'_, OrchestratorState>) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_session_delete(&storage.sessions, id)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_session_reorder(
    sessions: &Sessions,
    id: String,
    sort_order: u32,
) -> Result<(), WorkspaceError> {
    sessions
        .reorder(SessionId::from_string(id), sort_order)
        .await
        .map_err(map_store_err)
}

#[tauri::command]
pub async fn session_reorder(
    id: String,
    sort_order: u32,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_session_reorder(&storage.sessions, id, sort_order)
        .await
        .map_err(|e| format!("{e:?}"))
}

/// Session-creation draft (built by the `session_create` command from the client's flat args).
#[derive(Debug, Default)]
pub struct SessionCreateRequest {
    pub project_id: Option<String>,
    pub title: Option<String>,
    pub title_source: Option<String>,
    pub template_id: Option<String>,
}

fn parse_title_source(s: Option<&str>) -> TitleSource {
    match s {
        Some("branch") => TitleSource::Branch,
        Some("both") => TitleSource::Both,
        Some("custom") => TitleSource::Custom,
        _ => TitleSource::AgentTitle,
    }
}

fn new_session_draft(req: SessionCreateRequest) -> NewSession {
    NewSession {
        project_id: req.project_id.map(ProjectId::new),
        title_source: parse_title_source(req.title_source.as_deref()),
        title: req.title,
        template_id: req.template_id.map(LaunchTemplateId::from_string),
    }
}

#[cfg(test)]
pub async fn do_session_create(
    launch_templates: &LaunchTemplates,
    sessions: &Sessions,
    req: SessionCreateRequest,
) -> Result<Session, WorkspaceError> {
    create_session(new_session_draft(req), launch_templates, sessions)
        .await
        .map_err(map_store_err)
}

#[tauri::command]
pub async fn session_create(
    project_id: Option<String>,
    title: Option<String>,
    title_source: Option<String>,
    template_id: Option<String>,
    orchestrator: State<'_, OrchestratorState>,
    surfaces: State<'_, SurfaceState>,
) -> Result<SessionResponse, String> {
    let storage = storage_or_err(&orchestrator).map_err(|e| format!("{e:?}"))?;
    let draft = new_session_draft(SessionCreateRequest {
        project_id,
        title,
        title_source,
        template_id,
    });
    let session = open_session(
        draft,
        &storage.launch_templates,
        &storage.sessions,
        surfaces.api.as_ref(),
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    Ok(session_response(session))
}

pub async fn do_session_layout_set(
    sessions: &Sessions,
    session_id: String,
    layout_json: String,
) -> Result<(), WorkspaceError> {
    sessions
        .set_layout(SessionId::from_string(session_id), layout_json)
        .await
        .map_err(map_store_err)
}

pub async fn do_session_layout_get(
    sessions: &Sessions,
    session_id: String,
) -> Result<Option<String>, WorkspaceError> {
    sessions
        .get_layout(SessionId::from_string(session_id))
        .await
        .map_err(map_store_err)
}

// `id` (not `session_id`) so the IPC arg matches the SDK and the other session commands.
#[tauri::command]
pub async fn session_layout_set(
    id: String,
    layout_json: String,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_session_layout_set(&storage.sessions, id, layout_json)
        .await
        .map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub async fn session_layout_get(
    id: String,
    state: State<'_, OrchestratorState>,
) -> Result<Option<String>, String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_session_layout_get(&storage.sessions, id)
        .await
        .map_err(|e| format!("{e:?}"))
}

// ── command library ───────────────────────────────────────────────────────────

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

pub async fn do_command_list(commands: &Commands) -> Result<Vec<CommandResponse>, WorkspaceError> {
    let cmds = commands.list().await.map_err(map_store_err)?;
    Ok(cmds.into_iter().map(command_response).collect())
}

pub async fn do_command_get(
    commands: &Commands,
    id: String,
) -> Result<Option<CommandResponse>, WorkspaceError> {
    let cmd = commands.get(id).await.map_err(map_store_err)?;
    Ok(cmd.map(command_response))
}

#[tauri::command]
pub async fn command_get(
    id: String,
    state: State<'_, OrchestratorState>,
) -> Result<Option<CommandResponse>, String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_command_get(&storage.commands, id)
        .await
        .map_err(|e| format!("{e:?}"))
}

pub async fn do_command_delete(commands: &Commands, id: String) -> Result<(), WorkspaceError> {
    commands.delete(id).await.map_err(map_store_err)
}

#[tauri::command]
pub async fn command_delete(id: String, state: State<'_, OrchestratorState>) -> Result<(), String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_command_delete(&storage.commands, id)
        .await
        .map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub async fn command_list(
    state: State<'_, OrchestratorState>,
) -> Result<Vec<CommandResponse>, String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_command_list(&storage.commands)
        .await
        .map_err(|e| format!("{e:?}"))
}

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

pub async fn do_command_create(
    commands: &Commands,
    req: CreateCommandRequest,
) -> Result<CommandResponse, WorkspaceError> {
    let cmd = commands
        .create(NewCommand {
            name: req.name,
            origin: CommandOrigin::Custom,
            cli: req.cli,
            args: req.args,
            env: req.env,
        })
        .await
        .map_err(map_store_err)?;
    Ok(command_response(cmd))
}

#[tauri::command]
pub async fn command_create(
    req: CreateCommandRequest,
    state: State<'_, OrchestratorState>,
) -> Result<CommandResponse, String> {
    let storage = storage_or_err(&state).map_err(|e| format!("{e:?}"))?;
    do_command_create(&storage.commands, req)
        .await
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use orchestrator::infra::memory::MemoryBackend;

    fn fake_storage() -> Storage {
        Storage::in_memory(MemoryBackend::new())
    }

    /// Assert a serialized response carries exactly the camelCase keys the SDK type declares, so
    /// host response shapes can't silently drift from the `@tillerd/sdk` contract.
    fn assert_keys(value: &serde_json::Value, expected: &[&str]) {
        let obj = value.as_object().expect("response serializes to an object");
        let mut got: Vec<&str> = obj.keys().map(String::as_str).collect();
        got.sort_unstable();
        let mut want = expected.to_vec();
        want.sort_unstable();
        assert_eq!(got, want, "response keys drifted from the SDK contract");
    }

    #[tokio::test]
    async fn project_response_matches_sdk_project_shape() {
        let storage = fake_storage();
        let p = do_project_create(&storage.projects, Some("P".to_string()), None)
            .await
            .unwrap();
        assert_keys(
            &serde_json::to_value(p).unwrap(),
            &["id", "name", "sourceKind", "rootPath", "workspaceId"],
        );
    }

    #[tokio::test]
    async fn session_response_matches_sdk_session_shape() {
        let storage = fake_storage();
        let s = do_session_create(
            &storage.launch_templates,
            &storage.sessions,
            SessionCreateRequest {
                title: Some("S".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_keys(
            &serde_json::to_value(session_response(s)).unwrap(),
            &["id", "projectId", "title", "titleSource", "createdAt"],
        );
    }

    #[tokio::test]
    async fn command_response_matches_sdk_command_shape() {
        let storage = fake_storage();
        let c = do_command_create(
            &storage.commands,
            CreateCommandRequest {
                name: "c".to_string(),
                cli: "/c".to_string(),
                args: vec![],
                env: Default::default(),
            },
        )
        .await
        .unwrap();
        assert_keys(
            &serde_json::to_value(c).unwrap(),
            &["id", "name", "origin", "cli", "args", "env"],
        );
    }

    #[tokio::test]
    async fn project_create_delegates_to_store() {
        let storage = fake_storage();
        let result = do_project_create(&storage.projects, Some("MyProject".to_string()), None)
            .await
            .unwrap();
        assert_eq!(result.name, "MyProject");
        assert_eq!(result.source_kind, "blank");
    }

    #[tokio::test]
    async fn project_create_without_name_yields_a_blank_project() {
        let storage = fake_storage();
        let result = do_project_create(&storage.projects, None, None)
            .await
            .unwrap();
        assert_eq!(result.name, "");
        assert_eq!(result.source_kind, "blank");
    }

    #[tokio::test]
    async fn project_list_includes_created_projects() {
        let storage = fake_storage();
        let created = do_project_create(&storage.projects, Some("Listed".to_string()), None)
            .await
            .unwrap();
        let projects = do_project_list(&storage.projects, None).await.unwrap();
        assert!(projects
            .iter()
            .any(|p| p.id == created.id && p.name == "Listed"));
    }

    #[tokio::test]
    async fn session_list_returns_non_archived_sessions() {
        let storage = fake_storage();
        storage
            .sessions
            .create(NewSession::default(), None)
            .await
            .unwrap();

        let result = do_session_list(&storage.sessions, None).await.unwrap();
        // At least the one we just created; Unfiled sessions also present
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn session_layout_set_and_get_round_trip() {
        let storage = fake_storage();
        let sess = storage
            .sessions
            .create(NewSession::default(), None)
            .await
            .unwrap();
        let sid = sess.id.as_str().to_string();

        do_session_layout_set(
            &storage.sessions,
            sid.clone(),
            r#"{"type":"leaf"}"#.to_string(),
        )
        .await
        .unwrap();
        let blob = do_session_layout_get(&storage.sessions, sid).await.unwrap();
        assert_eq!(blob.as_deref(), Some(r#"{"type":"leaf"}"#));
    }

    #[tokio::test]
    async fn not_found_error_is_serialized() {
        let storage = fake_storage();
        let err = do_session_layout_set(
            &storage.sessions,
            "no-such-session".to_string(),
            "{}".to_string(),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[tokio::test]
    async fn unfiled_guard_is_serialized() {
        let storage = fake_storage();
        let err = storage
            .projects
            .archive(ProjectId::unfiled())
            .await
            .map_err(map_store_err)
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::UnfiledGuard { .. }));
    }

    #[tokio::test]
    async fn list_commands_returns_all_library_entries() {
        let storage = fake_storage();
        let cmds = do_command_list(&storage.commands).await.unwrap();
        assert!(!cmds.is_empty(), "prebuilt commands should be present");
    }

    #[tokio::test]
    async fn create_command_persists_custom_entry() {
        let storage = fake_storage();
        let req = CreateCommandRequest {
            name: "my-tool".to_string(),
            cli: "/usr/bin/tool".to_string(),
            args: vec!["--verbose".to_string()],
            env: Default::default(),
        };
        let result = do_command_create(&storage.commands, req).await.unwrap();
        assert_eq!(result.name, "my-tool");
        assert_eq!(result.origin, "custom");
    }

    async fn make_project(storage: &Storage, name: &str) -> ProjectId {
        storage
            .projects
            .create(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some(name.to_string()),
                workspace_id: None,
            })
            .await
            .unwrap()
            .id
    }

    #[tokio::test]
    async fn project_rename_reaches_the_store() {
        let storage = fake_storage();
        let id = make_project(&storage, "Old").await;
        do_project_rename(
            &storage.projects,
            id.as_str().to_string(),
            "New".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(
            storage
                .projects
                .get(id.clone())
                .await
                .unwrap()
                .unwrap()
                .name,
            "New"
        );
    }

    #[tokio::test]
    async fn project_rename_on_absent_is_not_found() {
        let storage = fake_storage();
        let err = do_project_rename(&storage.projects, "no-such".to_string(), "x".to_string())
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[tokio::test]
    async fn project_archive_reaches_the_store() {
        let storage = fake_storage();
        let id = make_project(&storage, "Doomed").await;
        do_project_archive(&storage.projects, id.as_str().to_string())
            .await
            .unwrap();
        assert!(storage.projects.get(id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn project_archive_on_absent_is_not_found() {
        let storage = fake_storage();
        let err = do_project_archive(&storage.projects, "no-such".to_string())
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[tokio::test]
    async fn project_delete_archives_then_purges_a_live_project() {
        let storage = fake_storage();
        let id = make_project(&storage, "Doomed").await;
        do_project_delete(&storage.projects, id.as_str().to_string())
            .await
            .unwrap();
        // Purged entirely: not in the active list and the row is gone.
        assert!(storage.projects.get(id.clone()).await.unwrap().is_none());
        assert!(!do_project_list(&storage.projects, None)
            .await
            .unwrap()
            .iter()
            .any(|p| p.id == id.as_str()));
    }

    #[tokio::test]
    async fn project_delete_on_absent_is_not_found() {
        let storage = fake_storage();
        let err = do_project_delete(&storage.projects, "no-such".to_string())
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[tokio::test]
    async fn project_reorder_reaches_the_store() {
        let storage = fake_storage();
        let id = make_project(&storage, "Movable").await;
        do_project_reorder(&storage.projects, id.as_str().to_string(), 5)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn project_reorder_on_absent_is_not_found() {
        let storage = fake_storage();
        let err = do_project_reorder(&storage.projects, "no-such".to_string(), 0)
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[tokio::test]
    async fn session_rename_reaches_the_store() {
        let storage = fake_storage();
        let sess = storage
            .sessions
            .create(NewSession::default(), None)
            .await
            .unwrap();
        do_session_rename(
            &storage.sessions,
            sess.id.as_str().to_string(),
            "Renamed".to_string(),
        )
        .await
        .unwrap();
        let listed = do_session_list(&storage.sessions, None).await.unwrap();
        assert!(listed
            .iter()
            .any(|s| s.id == sess.id.as_str() && s.title == "Renamed"));
    }

    #[tokio::test]
    async fn session_rename_on_absent_is_not_found() {
        let storage = fake_storage();
        let err = do_session_rename(&storage.sessions, "no-such".to_string(), "x".to_string())
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[tokio::test]
    async fn session_archive_reaches_the_store() {
        let storage = fake_storage();
        let sess = storage
            .sessions
            .create(NewSession::default(), None)
            .await
            .unwrap();
        do_session_archive(&storage.sessions, sess.id.as_str().to_string())
            .await
            .unwrap();
        let listed = do_session_list(&storage.sessions, None).await.unwrap();
        assert!(!listed.iter().any(|s| s.id == sess.id.as_str()));
    }

    #[tokio::test]
    async fn session_archive_on_absent_is_not_found() {
        let storage = fake_storage();
        let err = do_session_archive(&storage.sessions, "no-such".to_string())
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[tokio::test]
    async fn session_delete_archives_then_purges_a_live_session() {
        let storage = fake_storage();
        let sess = storage
            .sessions
            .create(NewSession::default(), None)
            .await
            .unwrap();
        do_session_delete(&storage.sessions, sess.id.as_str().to_string())
            .await
            .unwrap();
        assert!(storage
            .sessions
            .get(sess.id.clone())
            .await
            .unwrap()
            .is_none());
        assert!(!do_session_list(&storage.sessions, None)
            .await
            .unwrap()
            .iter()
            .any(|s| s.id == sess.id.as_str()));
    }

    #[tokio::test]
    async fn session_delete_on_absent_is_not_found() {
        let storage = fake_storage();
        let err = do_session_delete(&storage.sessions, "no-such".to_string())
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[tokio::test]
    async fn session_reorder_reaches_the_store() {
        let storage = fake_storage();
        let sess = storage
            .sessions
            .create(NewSession::default(), None)
            .await
            .unwrap();
        do_session_reorder(&storage.sessions, sess.id.as_str().to_string(), 3)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn session_reorder_on_absent_is_not_found() {
        let storage = fake_storage();
        let err = do_session_reorder(&storage.sessions, "no-such".to_string(), 0)
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[tokio::test]
    async fn command_get_returns_entry_then_none_after_delete() {
        let storage = fake_storage();
        let created = do_command_create(
            &storage.commands,
            CreateCommandRequest {
                name: "tool".to_string(),
                cli: "/tool".to_string(),
                args: vec![],
                env: Default::default(),
            },
        )
        .await
        .unwrap();

        let got = do_command_get(&storage.commands, created.id.clone())
            .await
            .unwrap();
        assert_eq!(got.map(|c| c.id), Some(created.id.clone()));

        do_command_delete(&storage.commands, created.id.clone())
            .await
            .unwrap();
        assert!(do_command_get(&storage.commands, created.id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn session_create_with_template_carries_the_spec_and_title() {
        let storage = fake_storage();
        let template = storage
            .launch_templates
            .create(orchestrator::entities::NewLaunchTemplate {
                project_id: ProjectId::unfiled(),
                spec_version: 1,
                spec_json: r#"{"version":1,"items":[]}"#.to_string(),
            })
            .await
            .unwrap();

        let session = do_session_create(
            &storage.launch_templates,
            &storage.sessions,
            SessionCreateRequest {
                template_id: Some(template.id.as_str().to_string()),
                title: Some("My Session".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(session.title, "My Session");
        let fetched = storage.sessions.get(session.id).await.unwrap().unwrap();
        assert_eq!(fetched.spec_version, Some(1));
    }

    // ── workspace ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn workspace_response_matches_sdk_workspace_shape() {
        let storage = fake_storage();
        let w = do_workspace_create(&storage.workspaces, "W".to_string())
            .await
            .unwrap();
        assert_keys(&serde_json::to_value(w).unwrap(), &["id", "name"]);
    }

    #[tokio::test]
    async fn workspace_create_and_list_reach_the_store() {
        let storage = fake_storage();
        let created = do_workspace_create(&storage.workspaces, "Mine".to_string())
            .await
            .unwrap();
        let listed = do_workspace_list(&storage.workspaces).await.unwrap();
        assert!(listed
            .iter()
            .any(|w| w.id == created.id && w.name == "Mine"));
    }

    #[tokio::test]
    async fn workspace_rename_reaches_the_store() {
        let storage = fake_storage();
        let created = do_workspace_create(&storage.workspaces, "old".to_string())
            .await
            .unwrap();
        do_workspace_rename(&storage.workspaces, created.id.clone(), "new".to_string())
            .await
            .unwrap();
        let listed = do_workspace_list(&storage.workspaces).await.unwrap();
        assert!(listed.iter().any(|w| w.id == created.id && w.name == "new"));
    }

    #[tokio::test]
    async fn workspace_delete_default_is_guarded() {
        let storage = fake_storage();
        let err = do_workspace_delete(&storage.workspaces, WorkspaceId::DEFAULT.to_string())
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::UnfiledGuard { .. }));
    }

    #[tokio::test]
    async fn project_move_reaches_the_store_and_scopes_list() {
        let storage = fake_storage();
        let ws = do_workspace_create(&storage.workspaces, "Target".to_string())
            .await
            .unwrap();
        let project = do_project_create(&storage.projects, Some("P".to_string()), None)
            .await
            .unwrap();
        do_project_move(&storage.projects, project.id.clone(), ws.id.clone())
            .await
            .unwrap();
        let scoped = do_project_list(&storage.projects, Some(ws.id))
            .await
            .unwrap();
        assert!(scoped.iter().any(|p| p.id == project.id));
    }

    #[tokio::test]
    async fn project_move_to_unknown_workspace_is_not_found() {
        let storage = fake_storage();
        let project = do_project_create(&storage.projects, Some("P".to_string()), None)
            .await
            .unwrap();
        let err = do_project_move(&storage.projects, project.id, "nope".to_string())
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }
}
