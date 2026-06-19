use super::*;

impl MemoryBackend {
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
}
