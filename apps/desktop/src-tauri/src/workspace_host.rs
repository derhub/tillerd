use std::collections::HashMap;
use std::sync::Arc;

use orchestrator::persistence::{
    Command, CommandOrigin, NewCommand, ProjectId, SessionId, SourceKind, Store,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::orchestrator_host::OrchestratorState;

// ── serializable response types ───────────────────────────────────────────────

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub id: String,
    pub project_id: String,
    pub title: String,
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
        ProjectNotFound(m) | SessionNotFound(m) | CommandNotFound(m) => {
            WorkspaceError::NotFound { message: m }
        }
        ProjectIsUnfiled => WorkspaceError::UnfiledGuard {
            message: "the Unfiled project cannot be modified".to_string(),
        },
        other => WorkspaceError::Internal {
            message: other.to_string(),
        },
    }
}

fn get_store(state: &OrchestratorState) -> Result<Arc<dyn Store>, WorkspaceError> {
    state.store_arc().ok_or_else(|| WorkspaceError::Internal {
        message: "orchestrator not ready".to_string(),
    })
}

// ── project ───────────────────────────────────────────────────────────────────

pub fn do_project_create(
    store: &Arc<dyn Store>,
    name: String,
) -> Result<ProjectResponse, WorkspaceError> {
    let project = store
        .create_project(orchestrator::persistence::NewProject {
            source_kind: SourceKind::Blank,
            root_path: None,
            name: Some(name),
        })
        .map_err(map_store_err)?;
    Ok(ProjectResponse {
        id: project.id.as_str().to_string(),
        name: project.name,
    })
}

#[tauri::command]
pub fn project_create(
    name: String,
    state: State<'_, OrchestratorState>,
) -> Result<ProjectResponse, String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_project_create(&store, name).map_err(|e| format!("{e:?}"))
}

pub fn do_project_rename(
    store: &Arc<dyn Store>,
    id: String,
    name: String,
) -> Result<(), WorkspaceError> {
    store
        .rename_project(&ProjectId::new(id), &name)
        .map_err(map_store_err)
}

#[tauri::command]
pub fn project_rename(
    id: String,
    name: String,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_project_rename(&store, id, name).map_err(|e| format!("{e:?}"))
}

pub fn do_project_archive(store: &Arc<dyn Store>, id: String) -> Result<(), WorkspaceError> {
    store
        .archive_project(&ProjectId::new(id))
        .map_err(map_store_err)
}

#[tauri::command]
pub fn project_archive(id: String, state: State<'_, OrchestratorState>) -> Result<(), String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_project_archive(&store, id).map_err(|e| format!("{e:?}"))
}

// ── session ───────────────────────────────────────────────────────────────────

pub fn do_session_list(
    store: &Arc<dyn Store>,
    project_id: Option<String>,
) -> Result<Vec<SessionResponse>, WorkspaceError> {
    let pid = project_id.map(ProjectId::new);
    let sessions = store.list_sessions(pid.as_ref()).map_err(map_store_err)?;
    Ok(sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id.as_str().to_string(),
            project_id: s.project_id.as_str().to_string(),
            title: s.title,
        })
        .collect())
}

#[tauri::command]
pub fn session_list(
    project_id: Option<String>,
    state: State<'_, OrchestratorState>,
) -> Result<Vec<SessionResponse>, String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_session_list(&store, project_id).map_err(|e| format!("{e:?}"))
}

pub fn do_session_rename(
    store: &Arc<dyn Store>,
    id: String,
    title: String,
) -> Result<(), WorkspaceError> {
    store
        .rename_session(&SessionId::from_string(id), &title)
        .map_err(map_store_err)
}

#[tauri::command]
pub fn session_rename(
    id: String,
    title: String,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_session_rename(&store, id, title).map_err(|e| format!("{e:?}"))
}

pub fn do_session_archive(store: &Arc<dyn Store>, id: String) -> Result<(), WorkspaceError> {
    store
        .archive_session(&SessionId::from_string(id))
        .map_err(map_store_err)
}

#[tauri::command]
pub fn session_archive(id: String, state: State<'_, OrchestratorState>) -> Result<(), String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_session_archive(&store, id).map_err(|e| format!("{e:?}"))
}

pub fn do_session_layout_set(
    store: &Arc<dyn Store>,
    session_id: String,
    layout_json: String,
) -> Result<(), WorkspaceError> {
    store
        .set_session_layout(&SessionId::from_string(session_id), &layout_json)
        .map_err(map_store_err)
}

pub fn do_session_layout_get(
    store: &Arc<dyn Store>,
    session_id: String,
) -> Result<Option<String>, WorkspaceError> {
    store
        .get_session_layout(&SessionId::from_string(session_id))
        .map_err(map_store_err)
}

#[tauri::command]
pub fn session_layout_set(
    session_id: String,
    layout_json: String,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_session_layout_set(&store, session_id, layout_json).map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub fn session_layout_get(
    session_id: String,
    state: State<'_, OrchestratorState>,
) -> Result<Option<String>, String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_session_layout_get(&store, session_id).map_err(|e| format!("{e:?}"))
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

pub fn do_command_list(store: &Arc<dyn Store>) -> Result<Vec<CommandResponse>, WorkspaceError> {
    let cmds = store.list_commands().map_err(map_store_err)?;
    Ok(cmds.into_iter().map(command_response).collect())
}

pub fn do_command_get(
    store: &Arc<dyn Store>,
    id: String,
) -> Result<Option<CommandResponse>, WorkspaceError> {
    let cmd = store.get_command(&id).map_err(map_store_err)?;
    Ok(cmd.map(command_response))
}

#[tauri::command]
pub fn command_get(
    id: String,
    state: State<'_, OrchestratorState>,
) -> Result<Option<CommandResponse>, String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_command_get(&store, id).map_err(|e| format!("{e:?}"))
}

pub fn do_command_delete(store: &Arc<dyn Store>, id: String) -> Result<(), WorkspaceError> {
    store.delete_command(&id).map_err(map_store_err)
}

#[tauri::command]
pub fn command_delete(id: String, state: State<'_, OrchestratorState>) -> Result<(), String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_command_delete(&store, id).map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub fn command_list(state: State<'_, OrchestratorState>) -> Result<Vec<CommandResponse>, String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_command_list(&store).map_err(|e| format!("{e:?}"))
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

pub fn do_command_create(
    store: &Arc<dyn Store>,
    req: CreateCommandRequest,
) -> Result<CommandResponse, WorkspaceError> {
    let cmd = store
        .create_command(NewCommand {
            name: req.name,
            origin: CommandOrigin::Custom,
            cli: req.cli,
            args: req.args,
            env: req.env,
        })
        .map_err(map_store_err)?;
    Ok(command_response(cmd))
}

#[tauri::command]
pub fn command_create(
    req: CreateCommandRequest,
    state: State<'_, OrchestratorState>,
) -> Result<CommandResponse, String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_command_create(&store, req).map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use orchestrator::persistence::memory::InMemoryStore;

    fn fake_store() -> Arc<dyn Store> {
        Arc::new(InMemoryStore::new())
    }

    #[test]
    fn project_create_delegates_to_store() {
        let store = fake_store();
        let result = do_project_create(&store, "MyProject".to_string()).unwrap();
        assert_eq!(result.name, "MyProject");
    }

    #[test]
    fn session_list_returns_non_archived_sessions() {
        let store = fake_store();
        store
            .create_session(orchestrator::persistence::NewSession::default())
            .unwrap();

        let result = do_session_list(&store, None).unwrap();
        // At least the one we just created; Unfiled sessions also present
        assert!(!result.is_empty());
    }

    #[test]
    fn session_layout_set_and_get_round_trip() {
        let store = fake_store();
        let sess = store
            .create_session(orchestrator::persistence::NewSession::default())
            .unwrap();
        let sid = sess.id.as_str().to_string();

        do_session_layout_set(&store, sid.clone(), r#"{"type":"leaf"}"#.to_string()).unwrap();
        let blob = do_session_layout_get(&store, sid).unwrap();
        assert_eq!(blob.as_deref(), Some(r#"{"type":"leaf"}"#));
    }

    #[test]
    fn not_found_error_is_serialized() {
        let store = fake_store();
        let err = do_session_layout_set(&store, "no-such-session".to_string(), "{}".to_string())
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[test]
    fn unfiled_guard_is_serialized() {
        let store = fake_store();
        let err = store
            .archive_project(&ProjectId::unfiled())
            .map_err(map_store_err)
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::UnfiledGuard { .. }));
    }

    #[test]
    fn list_commands_returns_all_library_entries() {
        let store = fake_store();
        let cmds = do_command_list(&store).unwrap();
        assert!(!cmds.is_empty(), "prebuilt commands should be present");
    }

    #[test]
    fn create_command_persists_custom_entry() {
        let store = fake_store();
        let req = CreateCommandRequest {
            name: "my-tool".to_string(),
            cli: "/usr/bin/tool".to_string(),
            args: vec!["--verbose".to_string()],
            env: Default::default(),
        };
        let result = do_command_create(&store, req).unwrap();
        assert_eq!(result.name, "my-tool");
        assert_eq!(result.origin, "custom");
    }

    fn make_project(store: &Arc<dyn Store>, name: &str) -> ProjectId {
        store
            .create_project(orchestrator::persistence::NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some(name.to_string()),
            })
            .unwrap()
            .id
    }

    #[test]
    fn project_rename_reaches_the_store() {
        let store = fake_store();
        let id = make_project(&store, "Old");
        do_project_rename(&store, id.as_str().to_string(), "New".to_string()).unwrap();
        assert_eq!(store.get_project(&id).unwrap().unwrap().name, "New");
    }

    #[test]
    fn project_rename_on_absent_is_not_found() {
        let store = fake_store();
        let err = do_project_rename(&store, "no-such".to_string(), "x".to_string()).unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[test]
    fn project_archive_reaches_the_store() {
        let store = fake_store();
        let id = make_project(&store, "Doomed");
        do_project_archive(&store, id.as_str().to_string()).unwrap();
        assert!(store.get_project(&id).unwrap().is_none());
    }

    #[test]
    fn project_archive_on_absent_is_not_found() {
        let store = fake_store();
        let err = do_project_archive(&store, "no-such".to_string()).unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[test]
    fn session_rename_reaches_the_store() {
        let store = fake_store();
        let sess = store
            .create_session(orchestrator::persistence::NewSession::default())
            .unwrap();
        do_session_rename(&store, sess.id.as_str().to_string(), "Renamed".to_string()).unwrap();
        let listed = do_session_list(&store, None).unwrap();
        assert!(listed
            .iter()
            .any(|s| s.id == sess.id.as_str() && s.title == "Renamed"));
    }

    #[test]
    fn session_rename_on_absent_is_not_found() {
        let store = fake_store();
        let err = do_session_rename(&store, "no-such".to_string(), "x".to_string()).unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[test]
    fn session_archive_reaches_the_store() {
        let store = fake_store();
        let sess = store
            .create_session(orchestrator::persistence::NewSession::default())
            .unwrap();
        do_session_archive(&store, sess.id.as_str().to_string()).unwrap();
        let listed = do_session_list(&store, None).unwrap();
        assert!(!listed.iter().any(|s| s.id == sess.id.as_str()));
    }

    #[test]
    fn session_archive_on_absent_is_not_found() {
        let store = fake_store();
        let err = do_session_archive(&store, "no-such".to_string()).unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[test]
    fn command_get_returns_entry_then_none_after_delete() {
        let store = fake_store();
        let created = do_command_create(
            &store,
            CreateCommandRequest {
                name: "tool".to_string(),
                cli: "/tool".to_string(),
                args: vec![],
                env: Default::default(),
            },
        )
        .unwrap();

        let got = do_command_get(&store, created.id.clone()).unwrap();
        assert_eq!(got.map(|c| c.id), Some(created.id.clone()));

        do_command_delete(&store, created.id.clone()).unwrap();
        assert!(do_command_get(&store, created.id).unwrap().is_none());
    }
}
