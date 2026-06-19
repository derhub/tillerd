use super::*;

impl MemoryBackend {
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
}
