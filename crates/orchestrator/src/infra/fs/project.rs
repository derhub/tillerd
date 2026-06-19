use super::*;

impl FsBackend {
    pub(crate) fn create_project(&self, draft: NewProject) -> Result<Project> {
        let mut state = self.state.write().unwrap();

        let ws_id = draft
            .workspace_id
            .clone()
            .unwrap_or_else(WorkspaceId::default_id);
        let ws_dir = state
            .get(ws_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::WorkspaceNotFound(ws_id.as_str().to_owned()))?;

        let proj_id = ProjectId::new(uuid::Uuid::new_v4().to_string());
        let name = draft
            .name
            .clone()
            .unwrap_or_else(|| infer_project_name(&draft));
        let proj_root = ws_dir.join("projects");
        create_dir_secure(&proj_root).map_err(persist)?;

        let sort_order = {
            let live = list_live_dirs(&proj_root)?;
            let max = live
                .iter()
                .filter_map(|d| self.dir_sort_order(d))
                .max()
                .unwrap_or(0);
            if live.is_empty() {
                0
            } else {
                max + 1
            }
        };

        let slug_base = slugify(&name, proj_id.as_str());
        let slug = unique_slug(&proj_root, &slug_base);
        let proj_dir = proj_root.join(&slug);
        create_dir_secure(&proj_dir).map_err(persist)?;

        let pf = ProjectFile {
            id: proj_id.as_str().to_owned(),
            name: name.clone(),
            source_kind: source_kind_str(draft.source_kind).to_owned(),
            root_path: draft.root_path.clone(),
            sort_order,
        };
        atomic_write(&proj_dir.join("project.json"), &to_json(&pf)?)?;
        state.insert(proj_id.as_str(), proj_dir);

        Ok(Project {
            id: proj_id,
            name,
            source_kind: draft.source_kind,
            root_path: draft.root_path,
            workspace_id: ws_id,
        })
    }

    pub(crate) fn rename_project(&self, id: &ProjectId, name: &str) -> Result<()> {
        if id.is_unfiled() {
            return Err(OrchestratorError::ProjectIsUnfiled);
        }
        let mut state = self.state.write().unwrap();
        let proj_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(id.as_str().to_owned()))?;

        let mut pf = read_json::<ProjectFile>(&proj_dir.join("project.json"))?;
        pf.name = name.to_owned();

        let slug_base = slugify(name, id.as_str());
        let proj_root = proj_dir
            .parent()
            .ok_or_else(|| OrchestratorError::Persistence("no parent".into()))?
            .to_path_buf();

        let current_slug = proj_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_owned();

        // Update name in file first (at current location).
        atomic_write(&proj_dir.join("project.json"), &to_json(&pf)?)?;

        if slug_base != current_slug {
            let new_slug = unique_slug(&proj_root, &slug_base);
            let new_proj_dir = proj_root.join(&new_slug);
            fs::rename(&proj_dir, &new_proj_dir).map_err(persist)?;
            reindex_subtree(&mut state, &new_proj_dir)?;
        }
        Ok(())
    }

    pub(crate) fn list_projects(&self, workspace_id: Option<&WorkspaceId>) -> Result<Vec<Project>> {
        let state = self.state.read().unwrap();

        // Collect workspace dirs to search.
        let ws_root = self.ws_root();
        let ws_dirs: Vec<PathBuf> = if let Some(ws_id) = workspace_id {
            let d = state
                .get(ws_id.as_str())
                .map(Path::to_path_buf)
                .ok_or_else(|| OrchestratorError::WorkspaceNotFound(ws_id.as_str().to_owned()))?;
            vec![d]
        } else {
            drop(state);
            list_live_dirs(&ws_root)?
        };

        let mut projects = Vec::new();
        for ws_dir in ws_dirs {
            let proj_root = ws_dir.join("projects");
            let mut proj_dirs = list_live_dirs(&proj_root)?;
            proj_dirs.sort_by_key(|d| {
                read_json::<ProjectFile>(&d.join("project.json"))
                    .map(|pf| pf.sort_order)
                    .unwrap_or(u32::MAX)
            });
            for pd in proj_dirs {
                if let Ok((_, proj)) = self.proj_from_file(&pd) {
                    projects.push(proj);
                }
            }
        }
        Ok(projects)
    }

    pub(crate) fn move_project(
        &self,
        project_id: &ProjectId,
        workspace_id: &WorkspaceId,
    ) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let proj_dir = state
            .get(project_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(project_id.as_str().to_owned()))?;
        let target_ws_dir = state
            .get(workspace_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| {
                OrchestratorError::WorkspaceNotFound(workspace_id.as_str().to_owned())
            })?;

        let slug = proj_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("proj")
            .to_owned();
        let target_proj_root = target_ws_dir.join("projects");
        create_dir_secure(&target_proj_root).map_err(persist)?;
        let new_slug = unique_slug(&target_proj_root, &slug);
        let new_proj_dir = target_proj_root.join(new_slug);
        fs::rename(&proj_dir, &new_proj_dir).map_err(persist)?;
        reindex_subtree(&mut state, &new_proj_dir)?;
        Ok(())
    }

    pub(crate) fn archive_project(&self, id: &ProjectId) -> Result<()> {
        if id.is_unfiled() {
            return Err(OrchestratorError::ProjectIsUnfiled);
        }
        let mut state = self.state.write().unwrap();
        let proj_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(id.as_str().to_owned()))?;

        if is_archived(&proj_dir) {
            return Err(OrchestratorError::ProjectNotFound(id.as_str().to_owned()));
        }

        let proj_root = proj_dir
            .parent()
            .ok_or_else(|| OrchestratorError::Persistence("no parent".into()))?
            .to_path_buf();
        let slug = proj_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("proj")
            .to_owned();
        let archive_root = proj_root.join(".archive");
        create_dir_secure(&archive_root).map_err(persist)?;
        // Disambiguate against entities already archived under the same slug,
        // else the rename collides with a non-empty dir (data loss / ENOTEMPTY).
        let target = archive_root.join(unique_slug(&archive_root, &slug));
        fs::rename(&proj_dir, &target).map_err(persist)?;
        reindex_subtree(&mut state, &target)?;
        Ok(())
    }

    pub(crate) fn hard_delete_project(&self, id: &ProjectId) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let proj_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(id.as_str().to_owned()))?;

        if !is_archived(&proj_dir) {
            return Err(OrchestratorError::ProjectNotArchived);
        }

        // Collect all ids to remove from index.
        let ids = collect_ids_in_subtree(&proj_dir)?;
        fs::remove_dir_all(&proj_dir).map_err(persist)?;
        for id_str in ids {
            state.remove(&id_str);
        }
        Ok(())
    }

    pub(crate) fn reorder_project(&self, id: &ProjectId, sort_order: u32) -> Result<()> {
        let _state = self.state.write().unwrap();
        let proj_dir = _state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(id.as_str().to_owned()))?;
        let mut pf = read_json::<ProjectFile>(&proj_dir.join("project.json"))?;
        pf.sort_order = sort_order;
        atomic_write(&proj_dir.join("project.json"), &to_json(&pf)?)
    }
}
