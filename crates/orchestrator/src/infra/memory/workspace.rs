use super::*;

impl MemoryBackend {
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
}
