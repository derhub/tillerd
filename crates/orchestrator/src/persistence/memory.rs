use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::Mutex;

use super::schema::current_version;
use super::{
    NewProject, NewSession, NewSurface, Project, ProjectId, Session, SessionId, SourceKind, Store,
    Surface, SurfaceId, TitleSource,
};
use crate::error::{OrchestratorError, Result};

pub struct InMemoryStore {
    inner: Mutex<Inner>,
}

struct Inner {
    version: u32,
    projects: HashMap<String, ProjectRecord>,
    sessions: HashMap<String, SessionRecord>,
    surfaces: HashMap<String, SurfaceRecord>,
}

#[derive(Clone)]
struct ProjectRecord {
    project: Project,
    deleted: bool,
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

impl InMemoryStore {
    pub fn new() -> Self {
        let mut projects = HashMap::new();
        projects.insert(
            ProjectId::UNFILED.to_string(),
            ProjectRecord {
                project: Project {
                    id: ProjectId::unfiled(),
                    name: "Unfiled".to_string(),
                    source_kind: SourceKind::Blank,
                    root_path: None,
                },
                deleted: false,
                created_seq: 0,
            },
        );
        Self {
            inner: Mutex::new(Inner {
                version: current_version(),
                projects,
                sessions: HashMap::new(),
                surfaces: HashMap::new(),
            }),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Store for InMemoryStore {
    fn schema_version(&self) -> Result<u32> {
        Ok(self.inner.lock().unwrap().version)
    }

    // ── project ───────────────────────────────────────────────────────────

    fn get_project(&self, id: &ProjectId) -> Result<Option<Project>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .projects
            .get(id.as_str())
            .filter(|r| !r.deleted)
            .map(|r| r.project.clone()))
    }

    fn create_project(&self, draft: NewProject) -> Result<Project> {
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

    fn rename_project(&self, id: &ProjectId, name: &str) -> Result<()> {
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

    fn list_projects(&self) -> Result<Vec<Project>> {
        let inner = self.inner.lock().unwrap();
        let mut records: Vec<&ProjectRecord> =
            inner.projects.values().filter(|r| !r.deleted).collect();
        records.sort_by_key(|r| Reverse(r.created_seq));
        Ok(records.into_iter().map(|r| r.project.clone()).collect())
    }

    fn archive_project(&self, id: &ProjectId) -> Result<()> {
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

    fn hard_delete_project(&self, id: &ProjectId) -> Result<()> {
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

    // ── session ───────────────────────────────────────────────────────────

    fn create_session(&self, draft: NewSession) -> Result<Session> {
        let session = Session {
            id: SessionId::mint(),
            project_id: draft.project_id.unwrap_or_else(ProjectId::unfiled),
            title: draft.title.unwrap_or_default(),
            title_source: draft.title_source,
            created_at: chrono_now(),
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

    fn rename_session(&self, id: &SessionId, title: &str) -> Result<()> {
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

    fn list_sessions(&self, project_id: Option<&ProjectId>) -> Result<Vec<Session>> {
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

    fn get_session(&self, id: &SessionId) -> Result<Option<Session>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .sessions
            .get(id.as_str())
            .filter(|r| !r.deleted)
            .map(|r| r.session.clone()))
    }

    fn archive_session(&self, id: &SessionId) -> Result<()> {
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

    fn hard_delete_session(&self, id: &SessionId) -> Result<()> {
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

    // ── surface ───────────────────────────────────────────────────────────

    fn create_surface(&self, draft: NewSurface) -> Result<Surface> {
        let surface = Surface {
            id: draft.id.unwrap_or_else(SurfaceId::mint),
            session_id: draft.session_id,
            kind: draft.kind,
            cwd: draft.cwd,
            last_status: None,
        };
        self.inner.lock().unwrap().surfaces.insert(
            surface.id.as_str().to_string(),
            SurfaceRecord {
                surface: surface.clone(),
                deleted: false,
            },
        );
        Ok(surface)
    }

    fn get_surface(&self, id: &SurfaceId) -> Result<Option<Surface>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .surfaces
            .get(id.as_str())
            .filter(|r| !r.deleted)
            .map(|r| r.surface.clone()))
    }

    fn list_resumable_surfaces(&self) -> Result<Vec<Surface>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
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
            .map(|r| r.surface.clone())
            .collect())
    }

    fn update_surface_status(&self, id: &SurfaceId, status: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(r) = inner.surfaces.get_mut(id.as_str()) {
            r.surface.last_status = Some(status.to_string());
        }
        Ok(())
    }

    fn soft_delete_surface(&self, id: &SurfaceId) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(r) = inner.surfaces.get_mut(id.as_str()) {
            r.deleted = true;
        }
        Ok(())
    }

    fn add_surface_to_session(&self, session_id: &SessionId, surface_id: &SurfaceId) -> Result<()> {
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

    fn remove_surface_from_session(
        &self,
        _session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()> {
        self.soft_delete_surface(surface_id)
    }

    // ── layout ────────────────────────────────────────────────────────────

    fn set_session_layout(&self, id: &SessionId, layout_json: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        match inner.sessions.get_mut(id.as_str()) {
            Some(r) if !r.deleted => {
                r.layout_json = Some(layout_json.to_string());
                Ok(())
            }
            _ => Err(OrchestratorError::SessionNotFound(id.as_str().to_string())),
        }
    }

    fn get_session_layout(&self, id: &SessionId) -> Result<Option<String>> {
        let inner = self.inner.lock().unwrap();
        match inner.sessions.get(id.as_str()) {
            Some(r) if !r.deleted => Ok(r.layout_json.clone()),
            _ => Err(OrchestratorError::SessionNotFound(id.as_str().to_string())),
        }
    }
}

fn infer_project_name(source: SourceKind, root_path: Option<&str>) -> Option<String> {
    match source {
        SourceKind::Blank => None,
        SourceKind::LocalDir | SourceKind::GitRepo | SourceKind::GitWorktree => {
            root_path.and_then(|p| {
                std::path::Path::new(p)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
        }
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
    use super::super::SurfaceKind;
    use super::*;

    #[test]
    fn fake_reports_current_schema_version() {
        let store = InMemoryStore::new();
        assert_eq!(store.schema_version().unwrap(), current_version());
    }

    #[test]
    fn fake_seeds_unfiled_and_resolves_sessions_to_it() {
        let store = InMemoryStore::new();
        assert!(store.get_project(&ProjectId::unfiled()).unwrap().is_some());

        let session = store.create_session(NewSession::default()).unwrap();
        assert_eq!(session.project_id, ProjectId::unfiled());
    }

    fn make_surface(store: &InMemoryStore) -> Surface {
        let session = store.create_session(NewSession::default()).unwrap();
        store
            .create_surface(NewSurface {
                id: None,
                session_id: session.id,
                kind: SurfaceKind::Terminal,
                cwd: None,
            })
            .unwrap()
    }

    #[test]
    fn create_then_get_surface_round_trips_including_last_status_none() {
        let store = InMemoryStore::new();

        let created = make_surface(&store);
        let fetched = store.get_surface(&created.id).unwrap().unwrap();

        assert_eq!(fetched, created);
        assert!(fetched.last_status.is_none());
    }

    #[test]
    fn list_resumable_surfaces_includes_a_created_surface() {
        let store = InMemoryStore::new();

        let created = make_surface(&store);
        let list = store.list_resumable_surfaces().unwrap();

        assert!(list.iter().any(|s| s.id == created.id));
    }

    #[test]
    fn soft_delete_excludes_surface_from_list_and_get() {
        let store = InMemoryStore::new();

        let surface = make_surface(&store);
        store.soft_delete_surface(&surface.id).unwrap();

        assert!(store.get_surface(&surface.id).unwrap().is_none());
        let list = store.list_resumable_surfaces().unwrap();
        assert!(!list.iter().any(|s| s.id == surface.id));
    }

    #[test]
    fn update_surface_status_is_reflected_by_get_surface() {
        let store = InMemoryStore::new();

        let surface = make_surface(&store);
        store.update_surface_status(&surface.id, "running").unwrap();

        let fetched = store.get_surface(&surface.id).unwrap().unwrap();
        assert_eq!(fetched.last_status.as_deref(), Some("running"));
    }
}
