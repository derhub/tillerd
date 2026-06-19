use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::Mutex;

use super::schema::current_version;
use crate::entities::{
    Command, CommandId, CommandOrigin, LaunchTemplate, LaunchTemplateId, NewCommand,
    NewLaunchTemplate, NewProject, NewSession, NewSurface, NewWorkspace, NotificationRecord,
    Project, ProjectId, Session, SessionId, SettingEntry, SettingScope, SourceKind, Surface,
    SurfaceId, TitleSource, Workspace, WorkspaceId,
};
use crate::error::{OrchestratorError, Result};

pub struct MemoryBackend {
    inner: Mutex<Inner>,
}

struct Inner {
    version: u32,
    workspaces: HashMap<String, WorkspaceRecord>,
    projects: HashMap<String, ProjectRecord>,
    sessions: HashMap<String, SessionRecord>,
    surfaces: HashMap<String, SurfaceRecord>,
    commands: HashMap<String, CommandRecord>,
    launch_templates: HashMap<String, LaunchTemplate>,
    /// Keyed by (scope, project_id, key) -> value_json, mirroring the sqlite primary key.
    settings: HashMap<(String, String, String), String>,
    /// Insertion-ordered notification history (oldest first); mirrors the sqlite rowid order.
    notifications: Vec<NotificationRecord>,
}

#[derive(Clone)]
struct CommandRecord {
    command: Command,
    deleted: bool,
}

#[derive(Clone)]
struct ProjectRecord {
    project: Project,
    deleted: bool,
    created_seq: u64,
}

#[derive(Clone)]
struct WorkspaceRecord {
    workspace: Workspace,
    sort_order: u32,
    created_seq: u64,
}

#[derive(Clone)]
struct SessionRecord {
    session: Session,
    deleted: bool,
    layout_json: Option<String>,
}

#[derive(Clone)]
struct SurfaceRecord {
    surface: Surface,
    deleted: bool,
}

impl MemoryBackend {
    pub fn new() -> Self {
        let mut workspaces = HashMap::new();
        workspaces.insert(
            WorkspaceId::DEFAULT.to_string(),
            WorkspaceRecord {
                workspace: Workspace {
                    id: WorkspaceId::default_id(),
                    name: "Default".to_string(),
                },
                sort_order: 0,
                created_seq: 0,
            },
        );
        let mut projects = HashMap::new();
        projects.insert(
            ProjectId::UNFILED.to_string(),
            ProjectRecord {
                project: Project {
                    id: ProjectId::unfiled(),
                    name: "Unfiled".to_string(),
                    source_kind: SourceKind::Blank,
                    root_path: None,
                    workspace_id: WorkspaceId::default_id(),
                },
                deleted: false,
                created_seq: 0,
            },
        );
        let store = Self {
            inner: Mutex::new(Inner {
                version: current_version(),
                workspaces,
                projects,
                sessions: HashMap::new(),
                surfaces: HashMap::new(),
                commands: HashMap::new(),
                launch_templates: HashMap::new(),
                settings: HashMap::new(),
                notifications: Vec::new(),
            }),
        };
        // Seed prebuilt commands on creation (idempotent).
        let _ = store.seed_commands();
        store
    }

    /// Materialize a session from a draft and a pre-resolved launch spec, with no
    /// template lookup. The backend primitive; template->spec resolution lives in the
    /// `create_session` coordinator.
    pub(crate) fn create_session_inner(
        &self,
        draft: NewSession,
        spec: Option<(u32, String)>,
    ) -> Result<Session> {
        let (spec_version, spec_json) = match spec {
            Some((version, json)) => (Some(version), Some(json)),
            None => (None, None),
        };
        let session = Session {
            id: SessionId::mint(),
            project_id: draft.project_id.unwrap_or_else(ProjectId::unfiled),
            title: draft.title.unwrap_or_default(),
            title_source: draft.title_source,
            created_at: chrono_now(),
            spec_version,
            spec_json,
        };
        self.inner.lock().unwrap().sessions.insert(
            session.id.as_str().to_string(),
            SessionRecord {
                session: session.clone(),
                deleted: false,
                layout_json: None,
            },
        );
        Ok(session)
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryBackend {
    pub(crate) fn schema_version(&self) -> Result<u32> {
        Ok(self.inner.lock().unwrap().version)
    }

    // ── project ───────────────────────────────────────────────────────────

    pub(crate) fn get_project(&self, id: &ProjectId) -> Result<Option<Project>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .projects
            .get(id.as_str())
            .filter(|r| !r.deleted)
            .map(|r| r.project.clone()))
    }

    pub(crate) fn create_project(&self, draft: NewProject) -> Result<Project> {
        let mut inner = self.inner.lock().unwrap();
        let seq = inner.projects.len() as u64;
        let id = ProjectId::new(uuid::Uuid::new_v4().to_string());
        let name = draft
            .name
            .or_else(|| infer_project_name(draft.source_kind, draft.root_path.as_deref()))
            .unwrap_or_default();
        let project = Project {
            id: id.clone(),
            name,
            source_kind: draft.source_kind,
            root_path: draft.root_path,
            workspace_id: draft.workspace_id.unwrap_or_else(WorkspaceId::default_id),
        };
        inner.projects.insert(
            id.as_str().to_string(),
            ProjectRecord {
                project: project.clone(),
                deleted: false,
                created_seq: seq,
            },
        );
        Ok(project)
    }

    pub(crate) fn rename_project(&self, id: &ProjectId, name: &str) -> Result<()> {
        if id.is_unfiled() {
            return Err(OrchestratorError::ProjectIsUnfiled);
        }
        let mut inner = self.inner.lock().unwrap();
        match inner.projects.get_mut(id.as_str()) {
            Some(r) if !r.deleted => {
                r.project.name = name.to_string();
                Ok(())
            }
            _ => Err(OrchestratorError::ProjectNotFound(id.as_str().to_string())),
        }
    }

    pub(crate) fn list_projects(&self, workspace_id: Option<&WorkspaceId>) -> Result<Vec<Project>> {
        let inner = self.inner.lock().unwrap();
        let mut records: Vec<&ProjectRecord> = inner
            .projects
            .values()
            .filter(|r| !r.deleted)
            .filter(|r| workspace_id.is_none_or(|w| r.project.workspace_id == *w))
            .collect();
        records.sort_by_key(|r| Reverse(r.created_seq));
        Ok(records.into_iter().map(|r| r.project.clone()).collect())
    }

    pub(crate) fn archive_project(&self, id: &ProjectId) -> Result<()> {
        if id.is_unfiled() {
            return Err(OrchestratorError::ProjectIsUnfiled);
        }
        let mut inner = self.inner.lock().unwrap();
        match inner.projects.get_mut(id.as_str()) {
            Some(r) if !r.deleted => {
                r.deleted = true;
            }
            _ => return Err(OrchestratorError::ProjectNotFound(id.as_str().to_string())),
        }
        // collect session ids to archive
        let sess_ids: Vec<String> = inner
            .sessions
            .values()
            .filter(|r| r.session.project_id == *id && !r.deleted)
            .map(|r| r.session.id.as_str().to_string())
            .collect();
        for sid in &sess_ids {
            if let Some(r) = inner.sessions.get_mut(sid) {
                r.deleted = true;
            }
            for surf in inner.surfaces.values_mut() {
                if surf.surface.session_id.as_str() == sid {
                    surf.deleted = true;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn hard_delete_project(&self, id: &ProjectId) -> Result<()> {
        if id.is_unfiled() {
            return Err(OrchestratorError::ProjectIsUnfiled);
        }
        let mut inner = self.inner.lock().unwrap();
        let rec = inner.projects.get(id.as_str());
        match rec {
            None => return Err(OrchestratorError::ProjectNotFound(id.as_str().to_string())),
            Some(r) if !r.deleted => return Err(OrchestratorError::ProjectNotArchived),
            _ => {}
        }
        // collect session ids for this project
        let sess_ids: Vec<String> = inner
            .sessions
            .values()
            .filter(|r| r.session.project_id == *id)
            .map(|r| r.session.id.as_str().to_string())
            .collect();
        for sid in &sess_ids {
            inner
                .surfaces
                .retain(|_, s| s.surface.session_id.as_str() != sid);
            inner.sessions.remove(sid);
        }
        inner.projects.remove(id.as_str());
        Ok(())
    }

    pub(crate) fn reorder_project(&self, id: &ProjectId, _sort_order: u32) -> Result<()> {
        let inner = self.inner.lock().unwrap();
        if !inner.projects.contains_key(id.as_str()) {
            return Err(OrchestratorError::ProjectNotFound(id.as_str().to_string()));
        }
        Ok(())
    }

    pub(crate) fn move_project(
        &self,
        project_id: &ProjectId,
        workspace_id: &WorkspaceId,
    ) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if !inner.workspaces.contains_key(workspace_id.as_str()) {
            return Err(OrchestratorError::WorkspaceNotFound(
                workspace_id.as_str().to_string(),
            ));
        }
        match inner.projects.get_mut(project_id.as_str()) {
            Some(r) if !r.deleted => {
                r.project.workspace_id = workspace_id.clone();
                Ok(())
            }
            _ => Err(OrchestratorError::ProjectNotFound(
                project_id.as_str().to_string(),
            )),
        }
    }

    // ── workspace ─────────────────────────────────────────────────────────

    pub(crate) fn create_workspace(&self, draft: NewWorkspace) -> Result<Workspace> {
        let mut inner = self.inner.lock().unwrap();
        let seq = inner.workspaces.len() as u64;
        let sort_order = inner
            .workspaces
            .values()
            .map(|r| r.sort_order)
            .max()
            .unwrap_or(0)
            + 1;
        let id = WorkspaceId::new(uuid::Uuid::new_v4().to_string());
        let workspace = Workspace {
            id: id.clone(),
            name: draft.name,
        };
        inner.workspaces.insert(
            id.as_str().to_string(),
            WorkspaceRecord {
                workspace: workspace.clone(),
                sort_order,
                created_seq: seq,
            },
        );
        Ok(workspace)
    }

    pub(crate) fn rename_workspace(&self, id: &WorkspaceId, name: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        match inner.workspaces.get_mut(id.as_str()) {
            Some(r) => {
                r.workspace.name = name.to_string();
                Ok(())
            }
            None => Err(OrchestratorError::WorkspaceNotFound(
                id.as_str().to_string(),
            )),
        }
    }

    pub(crate) fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let inner = self.inner.lock().unwrap();
        let mut records: Vec<&WorkspaceRecord> = inner.workspaces.values().collect();
        records.sort_by_key(|r| (r.sort_order, r.created_seq));
        Ok(records.into_iter().map(|r| r.workspace.clone()).collect())
    }

    pub(crate) fn reorder_workspace(&self, id: &WorkspaceId, sort_order: u32) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        match inner.workspaces.get_mut(id.as_str()) {
            Some(r) => {
                r.sort_order = sort_order;
                Ok(())
            }
            None => Err(OrchestratorError::WorkspaceNotFound(
                id.as_str().to_string(),
            )),
        }
    }

    pub(crate) fn delete_workspace(&self, id: &WorkspaceId) -> Result<()> {
        if id.is_default() {
            return Err(OrchestratorError::WorkspaceIsDefault);
        }
        let mut inner = self.inner.lock().unwrap();
        if inner.workspaces.remove(id.as_str()).is_none() {
            return Err(OrchestratorError::WorkspaceNotFound(
                id.as_str().to_string(),
            ));
        }
        let default = WorkspaceId::default_id();
        for r in inner.projects.values_mut() {
            if r.project.workspace_id == *id {
                r.project.workspace_id = default.clone();
            }
        }
        Ok(())
    }

    // ── session ───────────────────────────────────────────────────────────

    pub(crate) fn rename_session(&self, id: &SessionId, title: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        match inner.sessions.get_mut(id.as_str()) {
            Some(r) if !r.deleted => {
                r.session.title = title.to_string();
                r.session.title_source = TitleSource::Custom;
                Ok(())
            }
            _ => Err(OrchestratorError::SessionNotFound(id.as_str().to_string())),
        }
    }

    pub(crate) fn list_sessions(&self, project_id: Option<&ProjectId>) -> Result<Vec<Session>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .sessions
            .values()
            .filter(|r| {
                !r.deleted
                    && project_id
                        .map(|pid| r.session.project_id == *pid)
                        .unwrap_or(true)
            })
            .map(|r| r.session.clone())
            .collect())
    }

    pub(crate) fn get_session(&self, id: &SessionId) -> Result<Option<Session>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .sessions
            .get(id.as_str())
            .filter(|r| !r.deleted)
            .map(|r| r.session.clone()))
    }

    pub(crate) fn archive_session(&self, id: &SessionId) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        match inner.sessions.get_mut(id.as_str()) {
            Some(r) if !r.deleted => {
                r.deleted = true;
            }
            _ => return Err(OrchestratorError::SessionNotFound(id.as_str().to_string())),
        }
        let sid = id.as_str().to_string();
        for surf in inner.surfaces.values_mut() {
            if surf.surface.session_id.as_str() == sid {
                surf.deleted = true;
            }
        }
        Ok(())
    }

    pub(crate) fn hard_delete_session(&self, id: &SessionId) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let rec = inner.sessions.get(id.as_str());
        match rec {
            None => return Err(OrchestratorError::SessionNotFound(id.as_str().to_string())),
            Some(r) if !r.deleted => return Err(OrchestratorError::SessionNotArchived),
            _ => {}
        }
        let sid = id.as_str().to_string();
        inner
            .surfaces
            .retain(|_, s| s.surface.session_id.as_str() != sid);
        inner.sessions.remove(id.as_str());
        Ok(())
    }

    pub(crate) fn reorder_session(&self, id: &SessionId, _sort_order: u32) -> Result<()> {
        let inner = self.inner.lock().unwrap();
        if !inner.sessions.contains_key(id.as_str()) {
            return Err(OrchestratorError::SessionNotFound(id.as_str().to_string()));
        }
        Ok(())
    }

    // ── surface ───────────────────────────────────────────────────────────

    pub(crate) fn create_surface(&self, draft: NewSurface) -> Result<Surface> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(placement) = &draft.placement {
            let clash = inner.surfaces.values().any(|r| {
                !r.deleted
                    && r.surface.session_id == draft.session_id
                    && r.surface.placement.as_deref() == Some(placement.as_str())
            });
            if clash {
                return Err(OrchestratorError::SurfaceConflict(placement.clone()));
            }
        }
        let surface = Surface {
            id: draft.id.unwrap_or_else(SurfaceId::mint),
            session_id: draft.session_id,
            kind: draft.kind,
            cwd: draft.cwd,
            last_status: None,
            placement: draft.placement,
        };
        inner.surfaces.insert(
            surface.id.as_str().to_string(),
            SurfaceRecord {
                surface: surface.clone(),
                deleted: false,
            },
        );
        Ok(surface)
    }

    pub(crate) fn get_surface(&self, id: &SurfaceId) -> Result<Option<Surface>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .surfaces
            .get(id.as_str())
            .filter(|r| !r.deleted)
            .map(|r| r.surface.clone()))
    }

    pub(crate) fn find_session_surface_by_placement(
        &self,
        session_id: &SessionId,
        placement: &str,
    ) -> Result<Option<Surface>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .surfaces
            .values()
            .filter(|r| {
                !r.deleted
                    && r.surface.session_id.as_str() == session_id.as_str()
                    && r.surface.placement.as_deref() == Some(placement)
            })
            .map(|r| r.surface.clone())
            .next())
    }

    pub(crate) fn list_resumable_surfaces(&self) -> Result<Vec<Surface>> {
        let mut inner = self.inner.lock().unwrap();
        let ids: Vec<String> = inner
            .surfaces
            .values()
            .filter(|r| {
                !r.deleted
                    && inner
                        .sessions
                        .get(r.surface.session_id.as_str())
                        .map(|s| !s.deleted)
                        .unwrap_or(false)
            })
            .map(|r| r.surface.id.as_str().to_string())
            .collect();
        // Lazy-migrate null-placement rows so resume can bind by placement.
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(r) = inner.surfaces.get_mut(&id) {
                if r.surface.placement.is_none() {
                    r.surface.placement = Some(uuid::Uuid::new_v4().to_string());
                }
                out.push(r.surface.clone());
            }
        }
        Ok(out)
    }

    pub(crate) fn update_surface_status(&self, id: &SurfaceId, status: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(r) = inner.surfaces.get_mut(id.as_str()) {
            r.surface.last_status = Some(status.to_string());
        }
        Ok(())
    }

    pub(crate) fn soft_delete_surface(&self, id: &SurfaceId) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(r) = inner.surfaces.get_mut(id.as_str()) {
            r.deleted = true;
        }
        Ok(())
    }

    pub(crate) fn add_surface_to_session(
        &self,
        session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()> {
        let inner = self.inner.lock().unwrap();
        let surf = inner.surfaces.get(surface_id.as_str());
        match surf {
            Some(s) => {
                if s.surface.session_id != *session_id {
                    return Err(OrchestratorError::SurfaceConflict(
                        surface_id.as_str().to_string(),
                    ));
                }
                Ok(())
            }
            None => Ok(()),
        }
    }

    pub(crate) fn remove_surface_from_session(
        &self,
        _session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()> {
        self.soft_delete_surface(surface_id)
    }

    pub(crate) fn set_session_spec(
        &self,
        id: &SessionId,
        spec_version: u32,
        spec_json: &str,
    ) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        match inner.sessions.get_mut(id.as_str()) {
            Some(r) if !r.deleted => {
                r.session.spec_version = Some(spec_version);
                r.session.spec_json = Some(spec_json.to_string());
                Ok(())
            }
            _ => Err(OrchestratorError::SessionNotFound(id.as_str().to_string())),
        }
    }

    // ── layout ────────────────────────────────────────────────────────────

    pub(crate) fn set_session_layout(&self, id: &SessionId, layout_json: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        match inner.sessions.get_mut(id.as_str()) {
            Some(r) if !r.deleted => {
                r.layout_json = Some(layout_json.to_string());
                Ok(())
            }
            _ => Err(OrchestratorError::SessionNotFound(id.as_str().to_string())),
        }
    }

    pub(crate) fn get_session_layout(&self, id: &SessionId) -> Result<Option<String>> {
        let inner = self.inner.lock().unwrap();
        match inner.sessions.get(id.as_str()) {
            Some(r) if !r.deleted => Ok(r.layout_json.clone()),
            _ => Err(OrchestratorError::SessionNotFound(id.as_str().to_string())),
        }
    }

    // ── command library ───────────────────────────────────────────────────

    pub(crate) fn list_commands(&self) -> Result<Vec<Command>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .commands
            .values()
            .filter(|r| !r.deleted)
            .map(|r| r.command.clone())
            .collect())
    }

    pub(crate) fn get_command(&self, id: &str) -> Result<Option<Command>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .commands
            .get(id)
            .filter(|r| !r.deleted)
            .map(|r| r.command.clone()))
    }

    pub(crate) fn create_command(&self, draft: NewCommand) -> Result<Command> {
        let command = Command {
            id: CommandId::mint(),
            name: draft.name,
            origin: draft.origin,
            cli: draft.cli,
            args: draft.args,
            env: draft.env,
        };
        self.inner.lock().unwrap().commands.insert(
            command.id.as_str().to_string(),
            CommandRecord {
                command: command.clone(),
                deleted: false,
            },
        );
        Ok(command)
    }

    pub(crate) fn delete_command(&self, id: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(r) = inner.commands.get_mut(id) {
            r.deleted = true;
        }
        Ok(())
    }

    pub(crate) fn seed_commands(&self) -> Result<()> {
        let seeds = prebuilt_commands_mem();
        let mut inner = self.inner.lock().unwrap();
        for cmd in seeds {
            inner
                .commands
                .entry(cmd.id.as_str().to_string())
                .or_insert(CommandRecord {
                    command: cmd,
                    deleted: false,
                });
        }
        Ok(())
    }

    // ── launch template ───────────────────────────────────────────────────

    pub(crate) fn create_launch_template(
        &self,
        draft: NewLaunchTemplate,
    ) -> Result<LaunchTemplate> {
        let template = LaunchTemplate {
            id: LaunchTemplateId::mint(),
            project_id: draft.project_id,
            spec_version: draft.spec_version,
            spec_json: draft.spec_json,
        };
        self.inner
            .lock()
            .unwrap()
            .launch_templates
            .insert(template.id.as_str().to_string(), template.clone());
        Ok(template)
    }

    pub(crate) fn get_launch_template(
        &self,
        id: &LaunchTemplateId,
    ) -> Result<Option<LaunchTemplate>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .launch_templates
            .get(id.as_str())
            .cloned())
    }

    pub(crate) fn set_launch_template_spec(
        &self,
        id: &LaunchTemplateId,
        spec_version: u32,
        spec_json: &str,
    ) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        match inner.launch_templates.get_mut(id.as_str()) {
            Some(t) => {
                t.spec_version = spec_version;
                t.spec_json = spec_json.to_string();
                Ok(())
            }
            None => Err(OrchestratorError::LaunchTemplateNotFound(
                id.as_str().to_string(),
            )),
        }
    }

    // ── settings ──────────────────────────────────────────────────────────

    pub(crate) fn get_setting(&self, scope: &SettingScope, key: &str) -> Result<Option<String>> {
        let (scope_col, project_col) = scope.columns();
        Ok(self
            .inner
            .lock()
            .unwrap()
            .settings
            .get(&(
                scope_col.to_string(),
                project_col.to_string(),
                key.to_string(),
            ))
            .cloned())
    }

    /// Resolve a key for a project: the project-scoped value if present, else the global value.
    pub(crate) fn resolve_setting(
        &self,
        project_id: &ProjectId,
        key: &str,
    ) -> Result<Option<String>> {
        if let Some(v) = self.get_setting(&SettingScope::Project(project_id.clone()), key)? {
            return Ok(Some(v));
        }
        self.get_setting(&SettingScope::Global, key)
    }

    pub(crate) fn set_setting(
        &self,
        scope: &SettingScope,
        key: &str,
        value_json: &str,
    ) -> Result<()> {
        let (scope_col, project_col) = scope.columns();
        self.inner.lock().unwrap().settings.insert(
            (
                scope_col.to_string(),
                project_col.to_string(),
                key.to_string(),
            ),
            value_json.to_string(),
        );
        Ok(())
    }

    pub(crate) fn list_settings(&self, scope: &SettingScope) -> Result<Vec<SettingEntry>> {
        let (scope_col, project_col) = scope.columns();
        let inner = self.inner.lock().unwrap();
        let mut entries: Vec<SettingEntry> = inner
            .settings
            .iter()
            .filter(|((s, p, _), _)| s == scope_col && p == project_col)
            .map(|((_, _, key), value_json)| SettingEntry {
                key: key.clone(),
                value_json: value_json.clone(),
            })
            .collect();
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(entries)
    }

    // ── notifications (ADR-0031) ──────────────────────────────────────────

    pub(crate) fn insert_notification(&self, rec: &NotificationRecord) -> Result<()> {
        self.inner.lock().unwrap().notifications.push(rec.clone());
        Ok(())
    }

    pub(crate) fn list_notifications(&self, limit: u32) -> Result<Vec<NotificationRecord>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .notifications
            .iter()
            .rev()
            .take(limit as usize)
            .cloned()
            .collect())
    }

    pub(crate) fn prune_notifications(&self, keep: u32) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let len = inner.notifications.len();
        let keep = keep as usize;
        if len > keep {
            inner.notifications.drain(0..len - keep);
        }
        Ok(())
    }
}

fn prebuilt_commands_mem() -> Vec<Command> {
    vec![Command {
        id: CommandId::from_string("00000000-0000-0000-0000-000000000101"),
        name: "login-shell".to_string(),
        origin: CommandOrigin::Prebuilt,
        cli: "/bin/bash".to_string(),
        args: vec!["-l".to_string()],
        env: Default::default(),
    }]
}

fn infer_project_name(source: SourceKind, root_path: Option<&str>) -> Option<String> {
    match source {
        SourceKind::Blank => None,
        SourceKind::LocalDir | SourceKind::GitRepo => root_path.and_then(|p| {
            std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        }),
    }
}

fn chrono_now() -> String {
    // Minimal ISO-like timestamp without pulling in chrono
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::SurfaceKind;

    #[test]
    fn fake_reports_current_schema_version() {
        let store = MemoryBackend::new();
        assert_eq!(store.schema_version().unwrap(), current_version());
    }

    #[test]
    fn fake_seeds_unfiled_and_resolves_sessions_to_it() {
        let store = MemoryBackend::new();
        assert!(store.get_project(&ProjectId::unfiled()).unwrap().is_some());

        let session = store
            .create_session_inner(NewSession::default(), None)
            .unwrap();
        assert_eq!(session.project_id, ProjectId::unfiled());
    }

    fn make_surface(store: &MemoryBackend) -> Surface {
        let session = store
            .create_session_inner(NewSession::default(), None)
            .unwrap();
        store
            .create_surface(NewSurface {
                id: None,
                session_id: session.id,
                kind: SurfaceKind::Terminal,
                cwd: None,
                placement: None,
            })
            .unwrap()
    }

    #[test]
    fn create_then_get_surface_round_trips_including_last_status_none() {
        let store = MemoryBackend::new();

        let created = make_surface(&store);
        let fetched = store.get_surface(&created.id).unwrap().unwrap();

        assert_eq!(fetched, created);
        assert!(fetched.last_status.is_none());
    }

    #[test]
    fn list_resumable_surfaces_includes_a_created_surface() {
        let store = MemoryBackend::new();

        let created = make_surface(&store);
        let list = store.list_resumable_surfaces().unwrap();

        assert!(list.iter().any(|s| s.id == created.id));
    }

    #[test]
    fn soft_delete_excludes_surface_from_list_and_get() {
        let store = MemoryBackend::new();

        let surface = make_surface(&store);
        store.soft_delete_surface(&surface.id).unwrap();

        assert!(store.get_surface(&surface.id).unwrap().is_none());
        let list = store.list_resumable_surfaces().unwrap();
        assert!(!list.iter().any(|s| s.id == surface.id));
    }

    #[test]
    fn update_surface_status_is_reflected_by_get_surface() {
        let store = MemoryBackend::new();

        let surface = make_surface(&store);
        store.update_surface_status(&surface.id, "running").unwrap();

        let fetched = store.get_surface(&surface.id).unwrap().unwrap();
        assert_eq!(fetched.last_status.as_deref(), Some("running"));
    }

    #[test]
    fn set_launch_template_spec_on_absent_template_is_not_found() {
        let store = MemoryBackend::new();
        let err = store
            .set_launch_template_spec(
                &LaunchTemplateId::from_string("no-such-template"),
                2,
                r#"{"version":2,"items":[]}"#,
            )
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::LaunchTemplateNotFound(_)));
    }

    // ── settings ──────────────────────────────────────────────────────────

    #[test]
    fn global_setting_round_trips() {
        let store = MemoryBackend::new();
        store
            .set_setting(&SettingScope::Global, "theme", r#""dark""#)
            .unwrap();
        let got = store.get_setting(&SettingScope::Global, "theme").unwrap();
        assert_eq!(got.as_deref(), Some(r#""dark""#));
    }

    #[test]
    fn project_scoped_setting_round_trips() {
        let store = MemoryBackend::new();
        let scope = SettingScope::Project(ProjectId::unfiled());
        store
            .set_setting(&scope, "env", r#"{"FOO":"bar"}"#)
            .unwrap();
        let got = store.get_setting(&scope, "env").unwrap();
        assert_eq!(got.as_deref(), Some(r#"{"FOO":"bar"}"#));
        // The same key under global is independent.
        assert!(store
            .get_setting(&SettingScope::Global, "env")
            .unwrap()
            .is_none());
    }

    #[test]
    fn overwriting_a_setting_replaces_the_value() {
        let store = MemoryBackend::new();
        store
            .set_setting(&SettingScope::Global, "k", r#"1"#)
            .unwrap();
        store
            .set_setting(&SettingScope::Global, "k", r#"2"#)
            .unwrap();
        assert_eq!(
            store
                .get_setting(&SettingScope::Global, "k")
                .unwrap()
                .as_deref(),
            Some("2")
        );
        // No duplicate rows: exactly one entry for the key.
        let listed = store.list_settings(&SettingScope::Global).unwrap();
        assert_eq!(listed.iter().filter(|e| e.key == "k").count(), 1);
    }

    #[test]
    fn project_value_takes_precedence_over_global() {
        let store = MemoryBackend::new();
        let pid = ProjectId::unfiled();
        store
            .set_setting(&SettingScope::Global, "template", r#""g""#)
            .unwrap();
        store
            .set_setting(&SettingScope::Project(pid.clone()), "template", r#""p""#)
            .unwrap();
        let resolved = store.resolve_setting(&pid, "template").unwrap();
        assert_eq!(resolved.as_deref(), Some(r#""p""#));
    }

    #[test]
    fn resolve_falls_back_to_global_on_project_miss() {
        let store = MemoryBackend::new();
        let pid = ProjectId::unfiled();
        store
            .set_setting(&SettingScope::Global, "template", r#""g""#)
            .unwrap();
        let resolved = store.resolve_setting(&pid, "template").unwrap();
        assert_eq!(resolved.as_deref(), Some(r#""g""#));
    }

    #[test]
    fn unknown_key_resolves_to_absent() {
        let store = MemoryBackend::new();
        let resolved = store
            .resolve_setting(&ProjectId::unfiled(), "never-set")
            .unwrap();
        assert!(resolved.is_none());
    }

    #[test]
    fn list_returns_written_entries() {
        let store = MemoryBackend::new();
        store.set_setting(&SettingScope::Global, "a", "1").unwrap();
        store.set_setting(&SettingScope::Global, "b", "2").unwrap();
        let listed = store.list_settings(&SettingScope::Global).unwrap();
        assert_eq!(
            listed,
            vec![
                SettingEntry {
                    key: "a".to_string(),
                    value_json: "1".to_string()
                },
                SettingEntry {
                    key: "b".to_string(),
                    value_json: "2".to_string()
                },
            ]
        );
    }

    #[test]
    fn suppression_choice_is_recorded_and_read() {
        let store = MemoryBackend::new();
        store
            .set_setting(&SettingScope::Global, "confirm.close-surface", "true")
            .unwrap();
        let got = store
            .get_setting(&SettingScope::Global, "confirm.close-surface")
            .unwrap();
        assert_eq!(got.as_deref(), Some("true"));
    }

    #[test]
    fn unset_confirmation_reads_as_absent() {
        let store = MemoryBackend::new();
        let got = store
            .get_setting(&SettingScope::Global, "confirm.never-shown")
            .unwrap();
        assert!(got.is_none(), "absent confirmation is not suppressed");
    }
}
