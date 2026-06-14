use std::collections::HashMap;
use std::sync::Arc;

use orchestrator::persistence::{
    Command, CommandOrigin, LaunchTemplateId, NewCommand, NewSession, Project, ProjectId, Session,
    SessionId, SourceKind, Store, TitleSource,
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
}

fn project_response(p: Project) -> ProjectResponse {
    ProjectResponse {
        id: p.id.as_str().to_string(),
        name: p.name,
        source_kind: p.source_kind.as_str().to_string(),
        root_path: p.root_path,
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
    name: Option<String>,
) -> Result<ProjectResponse, WorkspaceError> {
    let project = store
        .create_project(orchestrator::persistence::NewProject {
            source_kind: SourceKind::Blank,
            root_path: None,
            name,
        })
        .map_err(map_store_err)?;
    Ok(project_response(project))
}

#[tauri::command]
pub fn project_create(
    name: Option<String>,
    state: State<'_, OrchestratorState>,
) -> Result<ProjectResponse, String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_project_create(&store, name).map_err(|e| format!("{e:?}"))
}

pub fn do_project_list(store: &Arc<dyn Store>) -> Result<Vec<ProjectResponse>, WorkspaceError> {
    let projects = store.list_projects().map_err(map_store_err)?;
    Ok(projects.into_iter().map(project_response).collect())
}

#[tauri::command]
pub fn project_list(state: State<'_, OrchestratorState>) -> Result<Vec<ProjectResponse>, String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_project_list(&store).map_err(|e| format!("{e:?}"))
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

/// Hard-delete a project. The store's `hard_delete_project` requires the project be archived first,
/// so a live project is archived (cascading its sessions/surfaces to soft-deleted) then purged in
/// one host call. An already-archived project skips straight to the purge.
pub fn do_project_delete(store: &Arc<dyn Store>, id: String) -> Result<(), WorkspaceError> {
    let pid = ProjectId::new(id);
    match store.archive_project(&pid) {
        Ok(()) => {}
        // Already archived (or absent): the purge below reports the definitive outcome.
        Err(orchestrator::OrchestratorError::ProjectNotFound(_)) => {}
        Err(e) => return Err(map_store_err(e)),
    }
    store.hard_delete_project(&pid).map_err(map_store_err)
}

#[tauri::command]
pub fn project_delete(id: String, state: State<'_, OrchestratorState>) -> Result<(), String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_project_delete(&store, id).map_err(|e| format!("{e:?}"))
}

pub fn do_project_reorder(
    store: &Arc<dyn Store>,
    id: String,
    sort_order: u32,
) -> Result<(), WorkspaceError> {
    store
        .reorder_project(&ProjectId::new(id), sort_order)
        .map_err(map_store_err)
}

#[tauri::command]
pub fn project_reorder(
    id: String,
    sort_order: u32,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_project_reorder(&store, id, sort_order).map_err(|e| format!("{e:?}"))
}

// ── session ───────────────────────────────────────────────────────────────────

pub fn do_session_list(
    store: &Arc<dyn Store>,
    project_id: Option<String>,
) -> Result<Vec<SessionResponse>, WorkspaceError> {
    let pid = project_id.map(ProjectId::new);
    let sessions = store.list_sessions(pid.as_ref()).map_err(map_store_err)?;
    Ok(sessions.into_iter().map(session_response).collect())
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

/// Hard-delete a session. Mirrors `do_project_delete`: archive first (the store's
/// `hard_delete_session` requires an archived row) then purge.
pub fn do_session_delete(store: &Arc<dyn Store>, id: String) -> Result<(), WorkspaceError> {
    let sid = SessionId::from_string(id);
    match store.archive_session(&sid) {
        Ok(()) => {}
        Err(orchestrator::OrchestratorError::SessionNotFound(_)) => {}
        Err(e) => return Err(map_store_err(e)),
    }
    store.hard_delete_session(&sid).map_err(map_store_err)
}

#[tauri::command]
pub fn session_delete(id: String, state: State<'_, OrchestratorState>) -> Result<(), String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_session_delete(&store, id).map_err(|e| format!("{e:?}"))
}

pub fn do_session_reorder(
    store: &Arc<dyn Store>,
    id: String,
    sort_order: u32,
) -> Result<(), WorkspaceError> {
    store
        .reorder_session(&SessionId::from_string(id), sort_order)
        .map_err(map_store_err)
}

#[tauri::command]
pub fn session_reorder(
    id: String,
    sort_order: u32,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_session_reorder(&store, id, sort_order).map_err(|e| format!("{e:?}"))
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

pub fn do_session_create(
    store: &Arc<dyn Store>,
    req: SessionCreateRequest,
) -> Result<Session, WorkspaceError> {
    store
        .create_session(NewSession {
            project_id: req.project_id.map(ProjectId::new),
            title_source: parse_title_source(req.title_source.as_deref()),
            title: req.title,
            template_id: req.template_id.map(LaunchTemplateId::from_string),
        })
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
    let store = get_store(&orchestrator).map_err(|e| format!("{e:?}"))?;
    let session = do_session_create(
        &store,
        SessionCreateRequest {
            project_id,
            title,
            title_source,
            template_id,
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    // Best-effort: instantiate the session's launch spec onto the runtime. A launch failure does
    // not undo the created session; per-item failures are recorded inside the executor's results.
    if let Err(e) = surfaces.api.launch_session(&session.id).await {
        eprintln!(
            "launch_session failed for {} (non-fatal): {e}",
            session.id.as_str()
        );
    }
    Ok(session_response(session))
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

// `id` (not `session_id`) so the IPC arg matches the SDK and the other session commands.
#[tauri::command]
pub fn session_layout_set(
    id: String,
    layout_json: String,
    state: State<'_, OrchestratorState>,
) -> Result<(), String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_session_layout_set(&store, id, layout_json).map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub fn session_layout_get(
    id: String,
    state: State<'_, OrchestratorState>,
) -> Result<Option<String>, String> {
    let store = get_store(&state).map_err(|e| format!("{e:?}"))?;
    do_session_layout_get(&store, id).map_err(|e| format!("{e:?}"))
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

    #[test]
    fn project_response_matches_sdk_project_shape() {
        let store = fake_store();
        let p = do_project_create(&store, Some("P".to_string())).unwrap();
        assert_keys(
            &serde_json::to_value(p).unwrap(),
            &["id", "name", "sourceKind", "rootPath"],
        );
    }

    #[test]
    fn session_response_matches_sdk_session_shape() {
        let store = fake_store();
        let s = do_session_create(
            &store,
            SessionCreateRequest {
                title: Some("S".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_keys(
            &serde_json::to_value(session_response(s)).unwrap(),
            &["id", "projectId", "title", "titleSource", "createdAt"],
        );
    }

    #[test]
    fn command_response_matches_sdk_command_shape() {
        let store = fake_store();
        let c = do_command_create(
            &store,
            CreateCommandRequest {
                name: "c".to_string(),
                cli: "/c".to_string(),
                args: vec![],
                env: Default::default(),
            },
        )
        .unwrap();
        assert_keys(
            &serde_json::to_value(c).unwrap(),
            &["id", "name", "origin", "cli", "args", "env"],
        );
    }

    #[test]
    fn project_create_delegates_to_store() {
        let store = fake_store();
        let result = do_project_create(&store, Some("MyProject".to_string())).unwrap();
        assert_eq!(result.name, "MyProject");
        assert_eq!(result.source_kind, "blank");
    }

    #[test]
    fn project_create_without_name_yields_a_blank_project() {
        let store = fake_store();
        let result = do_project_create(&store, None).unwrap();
        assert_eq!(result.name, "");
        assert_eq!(result.source_kind, "blank");
    }

    #[test]
    fn project_list_includes_created_projects() {
        let store = fake_store();
        let created = do_project_create(&store, Some("Listed".to_string())).unwrap();
        let projects = do_project_list(&store).unwrap();
        assert!(projects
            .iter()
            .any(|p| p.id == created.id && p.name == "Listed"));
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
    fn project_delete_archives_then_purges_a_live_project() {
        let store = fake_store();
        let id = make_project(&store, "Doomed");
        do_project_delete(&store, id.as_str().to_string()).unwrap();
        // Purged entirely: not in the active list and the row is gone.
        assert!(store.get_project(&id).unwrap().is_none());
        assert!(!do_project_list(&store)
            .unwrap()
            .iter()
            .any(|p| p.id == id.as_str()));
    }

    #[test]
    fn project_delete_on_absent_is_not_found() {
        let store = fake_store();
        let err = do_project_delete(&store, "no-such".to_string()).unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[test]
    fn project_reorder_reaches_the_store() {
        let store = fake_store();
        let id = make_project(&store, "Movable");
        do_project_reorder(&store, id.as_str().to_string(), 5).unwrap();
    }

    #[test]
    fn project_reorder_on_absent_is_not_found() {
        let store = fake_store();
        let err = do_project_reorder(&store, "no-such".to_string(), 0).unwrap_err();
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
    fn session_delete_archives_then_purges_a_live_session() {
        let store = fake_store();
        let sess = store
            .create_session(orchestrator::persistence::NewSession::default())
            .unwrap();
        do_session_delete(&store, sess.id.as_str().to_string()).unwrap();
        assert!(store.get_session(&sess.id).unwrap().is_none());
        assert!(!do_session_list(&store, None)
            .unwrap()
            .iter()
            .any(|s| s.id == sess.id.as_str()));
    }

    #[test]
    fn session_delete_on_absent_is_not_found() {
        let store = fake_store();
        let err = do_session_delete(&store, "no-such".to_string()).unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound { .. }));
    }

    #[test]
    fn session_reorder_reaches_the_store() {
        let store = fake_store();
        let sess = store
            .create_session(orchestrator::persistence::NewSession::default())
            .unwrap();
        do_session_reorder(&store, sess.id.as_str().to_string(), 3).unwrap();
    }

    #[test]
    fn session_reorder_on_absent_is_not_found() {
        let store = fake_store();
        let err = do_session_reorder(&store, "no-such".to_string(), 0).unwrap_err();
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

    #[test]
    fn session_create_with_template_carries_the_spec_and_title() {
        let store = fake_store();
        let template = store
            .create_launch_template(orchestrator::persistence::NewLaunchTemplate {
                project_id: ProjectId::unfiled(),
                spec_version: 1,
                spec_json: r#"{"version":1,"items":[]}"#.to_string(),
            })
            .unwrap();

        let session = do_session_create(
            &store,
            SessionCreateRequest {
                template_id: Some(template.id.as_str().to_string()),
                title: Some("My Session".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(session.title, "My Session");
        let fetched = store.get_session(&session.id).unwrap().unwrap();
        assert_eq!(fetched.spec_version, Some(1));
    }
}
