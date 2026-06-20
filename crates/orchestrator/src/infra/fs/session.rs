use super::*;

impl FsBackend {
    // ── session ───────────────────────────────────────────────────────────

    pub(crate) fn create_session(
        &self,
        draft: NewSession,
        spec: Option<(u32, String)>,
    ) -> Result<Session> {
        self.ensure_index()?;
        let mut state = self.state.write().unwrap();
        let proj_id = draft.project_id.clone().unwrap_or_else(ProjectId::unfiled);
        let proj_dir = state
            .get(proj_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::ProjectNotFound(proj_id.as_str().to_owned()))?;

        let sess_id = SessionId::mint();
        let title = draft.title.clone().unwrap_or_else(|| "Untitled".to_owned());
        let sess_root = proj_dir.join("sessions");
        create_dir_secure(&sess_root).map_err(persist)?;

        let sort_order = {
            let live = list_live_dirs(&sess_root)?;
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

        let slug_base = slugify(&title, sess_id.as_str());
        let slug = unique_slug(&sess_root, &slug_base);
        let sess_dir = sess_root.join(&slug);
        create_dir_secure(&sess_dir).map_err(persist)?;

        let (spec_version, spec_json) = match spec {
            Some((v, j)) => (Some(v), Some(j)),
            None => (None, None),
        };

        let created_at = now_iso8601();
        let sf = SessionFile {
            id: sess_id.as_str().to_owned(),
            title: title.clone(),
            title_source: title_source_str(draft.title_source).to_owned(),
            created_at: created_at.clone(),
            sort_order,
            spec_version,
            spec_json: spec_json.clone(),
        };
        self.write_file(&sess_dir.join("session.json"), &to_json(&sf)?)?;
        state.insert(sess_id.as_str(), sess_dir);

        Ok(Session {
            id: sess_id,
            project_id: proj_id,
            title,
            title_source: draft.title_source,
            created_at,
            spec_version: sf.spec_version,
            spec_json,
        })
    }

    pub(crate) fn rename_session(&self, id: &SessionId, title: &str) -> Result<()> {
        self.ensure_index()?;
        let mut state = self.state.write().unwrap();
        let sess_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(id.as_str().to_owned()))?;

        let mut sf = self
            .cache
            .read::<SessionFile>(&sess_dir.join("session.json"))?;
        sf.title = title.to_owned();
        sf.title_source = title_source_str(TitleSource::Custom).to_owned();

        let slug_base = slugify(title, id.as_str());
        let sess_root = sess_dir
            .parent()
            .ok_or_else(|| OrchestratorError::Persistence("no parent".into()))?
            .to_path_buf();
        let current_slug = sess_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_owned();

        self.write_file(&sess_dir.join("session.json"), &to_json(&sf)?)?;

        if slug_base != current_slug {
            let new_slug = unique_slug(&sess_root, &slug_base);
            let new_sess_dir = sess_root.join(&new_slug);
            fs::rename(&sess_dir, &new_sess_dir).map_err(persist)?;
            state.insert(id.as_str(), new_sess_dir);
        }
        Ok(())
    }

    pub(crate) fn list_sessions(&self, project_id: Option<&ProjectId>) -> Result<Vec<Session>> {
        self.ensure_index()?;
        let state = self.state.read().unwrap();

        // Collect project dirs to search.
        let proj_dirs: Vec<PathBuf> = if let Some(proj_id) = project_id {
            let d = state
                .get(proj_id.as_str())
                .map(Path::to_path_buf)
                .ok_or_else(|| OrchestratorError::ProjectNotFound(proj_id.as_str().to_owned()))?;
            if is_archived(&d) {
                return Ok(vec![]);
            }
            vec![d]
        } else {
            // All non-archived projects.
            let ws_root = self.ws_root();
            drop(state);
            let mut all = Vec::new();
            for ws_dir in list_live_dirs(&ws_root)? {
                for pd in list_live_dirs(&ws_dir.join("projects"))? {
                    all.push(pd);
                }
            }
            all
        };

        let mut sessions = Vec::new();
        for proj_dir in proj_dirs {
            let sess_root = proj_dir.join("sessions");
            let mut sess_dirs = list_live_dirs(&sess_root)?;
            sess_dirs.sort_by_key(|d| {
                self.cache
                    .read::<SessionFile>(&d.join("session.json"))
                    .map(|sf| sf.sort_order)
                    .unwrap_or(u32::MAX)
            });
            for sd in sess_dirs {
                if let Ok((_, sess)) = self.sess_from_file(&sd) {
                    sessions.push(sess);
                }
            }
        }
        Ok(sessions)
    }

    pub(crate) fn get_session(&self, id: &SessionId) -> Result<Option<Session>> {
        self.ensure_index()?;
        let state = self.state.read().unwrap();
        let sess_dir = match state.get(id.as_str()) {
            Some(p) => p.to_path_buf(),
            None => return Ok(None),
        };
        drop(state);
        if is_archived(&sess_dir) || !sess_dir.join("session.json").exists() {
            return Ok(None);
        }
        let (_, session) = self.sess_from_file(&sess_dir)?;
        Ok(Some(session))
    }

    pub(crate) fn archive_session(&self, id: &SessionId) -> Result<()> {
        self.ensure_index()?;
        let mut state = self.state.write().unwrap();
        let sess_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(id.as_str().to_owned()))?;

        if is_archived(&sess_dir) {
            return Err(OrchestratorError::SessionNotFound(id.as_str().to_owned()));
        }

        let sess_root = sess_dir
            .parent()
            .ok_or_else(|| OrchestratorError::Persistence("no parent".into()))?
            .to_path_buf();
        let slug = sess_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("session")
            .to_owned();
        let archive_root = sess_root.join(".archive");
        create_dir_secure(&archive_root).map_err(persist)?;
        // Disambiguate against sessions already archived under the same slug.
        let target = archive_root.join(unique_slug(&archive_root, &slug));
        fs::rename(&sess_dir, &target).map_err(persist)?;
        state.insert(id.as_str(), target);
        Ok(())
    }

    pub(crate) fn hard_delete_session(&self, id: &SessionId) -> Result<()> {
        self.ensure_index()?;
        let mut state = self.state.write().unwrap();
        let sess_dir = state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(id.as_str().to_owned()))?;

        if !is_archived(&sess_dir) {
            return Err(OrchestratorError::SessionNotArchived);
        }

        fs::remove_dir_all(&sess_dir).map_err(persist)?;
        state.remove(id.as_str());
        Ok(())
    }

    pub(crate) fn reorder_session(&self, id: &SessionId, sort_order: u32) -> Result<()> {
        self.ensure_index()?;
        let _state = self.state.write().unwrap();
        let sess_dir = _state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(id.as_str().to_owned()))?;
        let mut sf = self
            .cache
            .read::<SessionFile>(&sess_dir.join("session.json"))?;
        sf.sort_order = sort_order;
        self.write_file(&sess_dir.join("session.json"), &to_json(&sf)?)
    }

    pub(crate) fn set_session_spec(
        &self,
        id: &SessionId,
        spec_version: u32,
        spec_json: &str,
    ) -> Result<()> {
        self.ensure_index()?;
        let _state = self.state.write().unwrap();
        let sess_dir = _state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(id.as_str().to_owned()))?;
        let mut sf = self
            .cache
            .read::<SessionFile>(&sess_dir.join("session.json"))?;
        sf.spec_version = Some(spec_version);
        sf.spec_json = Some(spec_json.to_owned());
        self.write_file(&sess_dir.join("session.json"), &to_json(&sf)?)
    }

    pub(crate) fn set_session_layout(&self, id: &SessionId, layout_json: &str) -> Result<()> {
        self.ensure_index()?;
        let _state = self.state.write().unwrap();
        let sess_dir = _state
            .get(id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(id.as_str().to_owned()))?;
        // layout_json is an opaque blob — store it as the panel_tree field.
        // Parse into serde_json::Value so we can embed it in LayoutFile.
        let panel_tree: Option<serde_json::Value> = serde_json::from_str(layout_json).ok();
        // Preserve existing surface bindings.
        let existing = self.read_layout_file(&sess_dir)?;
        let lf = LayoutFile {
            panel_tree,
            surfaces: existing.surfaces,
        };
        self.write_file(&sess_dir.join("layout.json"), &to_json(&lf)?)
    }

    pub(crate) fn get_session_layout(&self, id: &SessionId) -> Result<Option<String>> {
        self.ensure_index()?;
        let state = self.state.read().unwrap();
        let sess_dir = match state.get(id.as_str()) {
            Some(p) => p.to_path_buf(),
            None => return Ok(None),
        };
        drop(state);
        let layout_path = sess_dir.join("layout.json");
        if !layout_path.exists() {
            return Ok(None);
        }
        let lf = self.cache.read::<LayoutFile>(&layout_path)?;
        match lf.panel_tree {
            Some(v) => {
                let s = serde_json::to_string(&v).map_err(persist)?;
                Ok(Some(s))
            }
            None => Ok(None),
        }
    }
}
