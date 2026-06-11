use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use super::schema::current_version;
use super::{
    NewSession, NewSurface, Project, ProjectId, Session, SessionId, SourceKind, Store, Surface,
    SurfaceId,
};
use crate::error::Result;

pub struct InMemoryStore {
    inner: Mutex<Inner>,
}

struct Inner {
    version: u32,
    projects: HashMap<String, Project>,
    sessions: HashMap<String, Session>,
    surfaces: HashMap<String, Surface>,
    deleted_surfaces: HashSet<String>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        let mut projects = HashMap::new();
        projects.insert(
            ProjectId::UNFILED.to_string(),
            Project {
                id: ProjectId::unfiled(),
                name: "Unfiled".to_string(),
                source_kind: SourceKind::Blank,
                root_path: None,
            },
        );
        Self {
            inner: Mutex::new(Inner {
                version: current_version(),
                projects,
                sessions: HashMap::new(),
                surfaces: HashMap::new(),
                deleted_surfaces: HashSet::new(),
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

    fn get_project(&self, id: &ProjectId) -> Result<Option<Project>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .projects
            .get(id.as_str())
            .cloned())
    }

    fn create_session(&self, draft: NewSession) -> Result<Session> {
        let session = Session {
            id: SessionId::mint(),
            project_id: draft.project.unwrap_or_else(ProjectId::unfiled),
            title: draft.title,
        };
        self.inner
            .lock()
            .unwrap()
            .sessions
            .insert(session.id.as_str().to_string(), session.clone());
        Ok(session)
    }

    fn create_surface(&self, draft: NewSurface) -> Result<Surface> {
        let surface = Surface {
            id: draft.id.unwrap_or_else(SurfaceId::mint),
            session_id: draft.session_id,
            kind: draft.kind,
            cwd: draft.cwd,
            last_status: None,
        };
        self.inner
            .lock()
            .unwrap()
            .surfaces
            .insert(surface.id.as_str().to_string(), surface.clone());
        Ok(surface)
    }

    fn get_surface(&self, id: &SurfaceId) -> Result<Option<Surface>> {
        let inner = self.inner.lock().unwrap();
        if inner.deleted_surfaces.contains(id.as_str()) {
            return Ok(None);
        }
        Ok(inner.surfaces.get(id.as_str()).cloned())
    }

    fn list_resumable_surfaces(&self) -> Result<Vec<Surface>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .surfaces
            .values()
            .filter(|s| !inner.deleted_surfaces.contains(s.id.as_str()))
            .cloned()
            .collect())
    }

    fn update_surface_status(&self, id: &SurfaceId, status: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(surface) = inner.surfaces.get_mut(id.as_str()) {
            surface.last_status = Some(status.to_string());
        }
        Ok(())
    }

    fn soft_delete_surface(&self, id: &SurfaceId) -> Result<()> {
        self.inner
            .lock()
            .unwrap()
            .deleted_surfaces
            .insert(id.as_str().to_string());
        Ok(())
    }
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
