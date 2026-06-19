use super::*;

impl FsBackend {
    // ── workspace ─────────────────────────────────────────────────────────

    pub(crate) fn create_workspace(&self, draft: NewWorkspace) -> Result<Workspace> {
        let mut state = self.state.write().unwrap();
        let id = WorkspaceId::new(uuid::Uuid::new_v4().to_string());
        let ws_root = self.ws_root();
        let sort_order = {
            let live = list_live_dirs(&ws_root)?;
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
        let slug_base = slugify(&draft.name, id.as_str());
        let slug = unique_slug(&ws_root, &slug_base);
        let ws_dir = ws_root.join(&slug);
        create_dir_secure(&ws_dir).map_err(persist)?;

        let wf = WorkspaceFile {
            id: id.as_str().to_owned(),
            name: draft.name.clone(),
            sort_order,
        };
        atomic_write(&ws_dir.join("workspace.json"), &to_json(&wf)?)?;
        state.insert(id.as_str(), ws_dir);
        Ok(Workspace {
            id,
            name: draft.name,
        })
    }

    pub(crate) fn rename_workspace(&self, id: &WorkspaceId, name: &str) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let ws_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::WorkspaceNotFound(id.as_str().to_owned()))?;

        let mut wf = read_json::<WorkspaceFile>(&ws_dir.join("workspace.json"))?;
        wf.name = name.to_owned();

        let slug_base = slugify(name, id.as_str());
        let ws_root = self.ws_root();
        // If slug would change, move the directory.
        let current_slug = ws_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_owned();
        let new_slug = if slug_base != current_slug {
            unique_slug(&ws_root, &slug_base)
        } else {
            current_slug.clone()
        };

        // Write updated file first (into the current location).
        atomic_write(&ws_dir.join("workspace.json"), &to_json(&wf)?)?;

        if new_slug != current_slug {
            let new_ws_dir = ws_root.join(&new_slug);
            fs::rename(&ws_dir, &new_ws_dir).map_err(persist)?;
            // Update index: workspace id + all nested project/session ids.
            reindex_subtree(&mut state, &new_ws_dir)?;
        }
        Ok(())
    }

    pub(crate) fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let ws_root = self.ws_root();
        let mut dirs = list_live_dirs(&ws_root)?;
        // Sort by sortOrder ascending.
        dirs.sort_by_key(|d| {
            read_json::<WorkspaceFile>(&d.join("workspace.json"))
                .map(|wf| wf.sort_order)
                .unwrap_or(u32::MAX)
        });
        let mut result = Vec::new();
        for d in dirs {
            let wf = read_json::<WorkspaceFile>(&d.join("workspace.json"))?;
            result.push(Workspace {
                id: WorkspaceId::new(wf.id),
                name: wf.name,
            });
        }
        Ok(result)
    }

    pub(crate) fn reorder_workspace(&self, id: &WorkspaceId, sort_order: u32) -> Result<()> {
        let _state = self.state.write().unwrap();
        let ws_dir = _state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::WorkspaceNotFound(id.as_str().to_owned()))?;
        let mut wf = read_json::<WorkspaceFile>(&ws_dir.join("workspace.json"))?;
        wf.sort_order = sort_order;
        atomic_write(&ws_dir.join("workspace.json"), &to_json(&wf)?)
    }

    pub(crate) fn delete_workspace(&self, id: &WorkspaceId) -> Result<()> {
        if id.as_str() == WorkspaceId::DEFAULT {
            return Err(OrchestratorError::WorkspaceIsDefault);
        }
        let mut state = self.state.write().unwrap();
        let ws_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::WorkspaceNotFound(id.as_str().to_owned()))?;

        // Reassign projects to Default workspace.
        let default_ws_dir = state
            .get(WorkspaceId::DEFAULT)
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::WorkspaceNotFound(WorkspaceId::DEFAULT.to_owned()))?;
        let default_proj_root = default_ws_dir.join("projects");
        create_dir_secure(&default_proj_root).map_err(persist)?;

        let proj_root = ws_dir.join("projects");
        if proj_root.exists() {
            for dir in list_live_dirs(&proj_root)? {
                let slug = dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("proj")
                    .to_owned();
                let target_slug = unique_slug(&default_proj_root, &slug);
                let target = default_proj_root.join(&target_slug);
                fs::rename(&dir, &target).map_err(persist)?;
                reindex_subtree(&mut state, &target)?;
            }
        }

        // Reassign archived projects too — they are still the workspace's
        // projects, and remove_dir_all below would otherwise destroy them.
        let archive_src = proj_root.join(".archive");
        if archive_src.exists() {
            let default_archive = default_proj_root.join(".archive");
            create_dir_secure(&default_archive).map_err(persist)?;
            for dir in list_live_dirs(&archive_src)? {
                let slug = dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("proj")
                    .to_owned();
                let target = default_archive.join(unique_slug(&default_archive, &slug));
                fs::rename(&dir, &target).map_err(persist)?;
                reindex_subtree(&mut state, &target)?;
            }
        }

        // Remove the workspace subtree.
        fs::remove_dir_all(&ws_dir).map_err(persist)?;
        state.remove(id.as_str());
        Ok(())
    }

    // ── project ───────────────────────────────────────────────────────────

    pub(crate) fn get_project(&self, id: &ProjectId) -> Result<Option<Project>> {
        let state = self.state.read().unwrap();
        let proj_dir = match state.get(id.as_str()) {
            Some(p) => p.to_path_buf(),
            None => return Ok(None),
        };
        drop(state);
        if is_archived(&proj_dir) || !proj_dir.join("project.json").exists() {
            return Ok(None);
        }
        let (_, project) = self.proj_from_file(&proj_dir)?;
        Ok(Some(project))
    }
}
