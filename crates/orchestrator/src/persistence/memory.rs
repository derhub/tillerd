use std::collections::HashMap;
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
            id: SurfaceId::mint(),
            session_id: draft.session_id,
            kind: draft.kind,
            cwd: draft.cwd,
        };
        self.inner
            .lock()
            .unwrap()
            .surfaces
            .insert(surface.id.as_str().to_string(), surface.clone());
        Ok(surface)
    }
}

#[cfg(test)]
mod tests {
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
}
