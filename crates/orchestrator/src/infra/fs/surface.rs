use super::*;

impl FsBackend {
    // ── surface ───────────────────────────────────────────────────────────

    pub(crate) fn create_surface(&self, draft: NewSurface) -> Result<Surface> {
        let state = self.state.write().unwrap();
        let sess_id = draft.session_id.clone();
        let sess_dir = state
            .get(sess_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(sess_id.as_str().to_owned()))?;

        let surf_id = match draft.id {
            Some(id) => id,
            None => SurfaceId::mint(),
        };

        // Placement uniqueness check (D6).
        if let Some(ref placement) = draft.placement {
            let lf = self.read_layout_file(&sess_dir)?;
            for binding in &lf.surfaces {
                if !binding.deleted && binding.placement.as_deref() == Some(placement.as_str()) {
                    return Err(OrchestratorError::SurfaceConflict(
                        surf_id.as_str().to_owned(),
                    ));
                }
            }
        }

        let binding = SurfaceBinding {
            id: surf_id.as_str().to_owned(),
            kind: surface_kind_str(draft.kind).to_owned(),
            placement: draft.placement.clone(),
            cwd: draft.cwd.clone(),
            last_status: None,
            deleted: false,
        };

        let mut lf = self.read_layout_file(&sess_dir)?;
        lf.surfaces.push(binding);
        atomic_write(&sess_dir.join("layout.json"), &to_json(&lf)?)?;
        drop(state);

        Ok(Surface {
            id: surf_id,
            session_id: sess_id,
            kind: draft.kind,
            cwd: draft.cwd,
            last_status: None,
            placement: draft.placement,
        })
    }

    pub(crate) fn get_surface(&self, id: &SurfaceId) -> Result<Option<Surface>> {
        // Must scan all live sessions.
        let ws_root = self.ws_root();
        for ws_dir in list_live_dirs(&ws_root)? {
            for proj_dir in list_live_dirs(&ws_dir.join("projects"))? {
                for sess_dir in list_live_dirs(&proj_dir.join("sessions"))? {
                    let lf = self.read_layout_file(&sess_dir)?;
                    for b in &lf.surfaces {
                        if b.id == id.as_str() && !b.deleted {
                            let sess_file =
                                read_json::<SessionFile>(&sess_dir.join("session.json"))?;
                            let kind = surface_kind_from_str(&b.kind)?;
                            return Ok(Some(Surface {
                                id: SurfaceId::from_string(b.id.clone()),
                                session_id: SessionId::from_string(sess_file.id),
                                kind,
                                cwd: b.cwd.clone(),
                                last_status: b.last_status.clone(),
                                placement: b.placement.clone(),
                            }));
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    pub(crate) fn find_session_surface_by_placement(
        &self,
        session_id: &SessionId,
        placement: &str,
    ) -> Result<Option<Surface>> {
        let state = self.state.read().unwrap();
        let sess_dir = match state.get(session_id.as_str()) {
            Some(p) => p.to_path_buf(),
            None => return Ok(None),
        };
        drop(state);
        let lf = self.read_layout_file(&sess_dir)?;
        for b in &lf.surfaces {
            if !b.deleted && b.placement.as_deref() == Some(placement) {
                let kind = surface_kind_from_str(&b.kind)?;
                return Ok(Some(Surface {
                    id: SurfaceId::from_string(b.id.clone()),
                    session_id: session_id.clone(),
                    kind,
                    cwd: b.cwd.clone(),
                    last_status: b.last_status.clone(),
                    placement: b.placement.clone(),
                }));
            }
        }
        Ok(None)
    }

    pub(crate) fn list_resumable_surfaces(&self) -> Result<Vec<Surface>> {
        let ws_root = self.ws_root();
        let mut surfaces = Vec::new();
        for ws_dir in list_live_dirs(&ws_root)? {
            for proj_dir in list_live_dirs(&ws_dir.join("projects"))? {
                for sess_dir in list_live_dirs(&proj_dir.join("sessions"))? {
                    let sf = read_json::<SessionFile>(&sess_dir.join("session.json"))?;
                    let lf = self.read_layout_file(&sess_dir)?;
                    for b in &lf.surfaces {
                        if !b.deleted {
                            if let Ok(kind) = surface_kind_from_str(&b.kind) {
                                surfaces.push(Surface {
                                    id: SurfaceId::from_string(b.id.clone()),
                                    session_id: SessionId::from_string(sf.id.clone()),
                                    kind,
                                    cwd: b.cwd.clone(),
                                    last_status: b.last_status.clone(),
                                    placement: b.placement.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
        Ok(surfaces)
    }

    pub(crate) fn update_surface_status(&self, id: &SurfaceId, status: &str) -> Result<()> {
        let _state = self.state.write().unwrap();
        let ws_root = self.ws_root();
        for ws_dir in list_live_dirs(&ws_root)? {
            for proj_dir in list_live_dirs(&ws_dir.join("projects"))? {
                for sess_dir in list_live_dirs(&proj_dir.join("sessions"))? {
                    let mut lf = self.read_layout_file(&sess_dir)?;
                    let mut changed = false;
                    for b in &mut lf.surfaces {
                        if b.id == id.as_str() && !b.deleted {
                            b.last_status = Some(status.to_owned());
                            changed = true;
                        }
                    }
                    if changed {
                        return atomic_write(&sess_dir.join("layout.json"), &to_json(&lf)?);
                    }
                }
            }
        }
        Ok(()) // Surface not found — silently ignore (matches Store trait behaviour)
    }

    pub(crate) fn soft_delete_surface(&self, id: &SurfaceId) -> Result<()> {
        let _state = self.state.write().unwrap();
        let ws_root = self.ws_root();
        for ws_dir in list_live_dirs(&ws_root)? {
            for proj_dir in list_live_dirs(&ws_dir.join("projects"))? {
                for sess_dir in list_live_dirs(&proj_dir.join("sessions"))? {
                    let mut lf = self.read_layout_file(&sess_dir)?;
                    let mut changed = false;
                    for b in &mut lf.surfaces {
                        if b.id == id.as_str() && !b.deleted {
                            b.deleted = true;
                            changed = true;
                        }
                    }
                    if changed {
                        return atomic_write(&sess_dir.join("layout.json"), &to_json(&lf)?);
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn add_surface_to_session(
        &self,
        session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()> {
        let _state = self.state.write().unwrap();
        let sess_dir = _state
            .get(session_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(session_id.as_str().to_owned()))?;

        let mut lf = self.read_layout_file(&sess_dir)?;

        // Check if this surface is already bound to a different session (conflict).
        // Find existing binding across the whole tree.
        let ws_root = self.ws_root();
        'outer: for ws_dir in list_live_dirs(&ws_root)? {
            for proj_dir in list_live_dirs(&ws_dir.join("projects"))? {
                for sd in list_live_dirs(&proj_dir.join("sessions"))? {
                    let other_lf = self.read_layout_file(&sd)?;
                    for b in &other_lf.surfaces {
                        if b.id == surface_id.as_str() && !b.deleted {
                            // Check whether it belongs to THIS session.
                            let sf = read_json::<SessionFile>(&sd.join("session.json"))?;
                            if sf.id != session_id.as_str() {
                                return Err(OrchestratorError::SurfaceConflict(
                                    surface_id.as_str().to_owned(),
                                ));
                            }
                            // Already in this session — no-op.
                            break 'outer;
                        }
                    }
                }
            }
        }

        // Check placement uniqueness for the new binding.
        // We need the binding details from somewhere — if the surface doesn't exist yet in
        // any session, it won't have a placement. We just add a minimal binding here.
        // If placement conflicts within this session:
        let binding = SurfaceBinding {
            id: surface_id.as_str().to_owned(),
            kind: "terminal".to_owned(), // default; updated by caller via create_surface
            placement: None,
            cwd: None,
            last_status: None,
            deleted: false,
        };
        lf.surfaces.push(binding);
        atomic_write(&sess_dir.join("layout.json"), &to_json(&lf)?)
    }

    pub(crate) fn remove_surface_from_session(
        &self,
        session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()> {
        let _state = self.state.write().unwrap();
        let sess_dir = _state
            .get(session_id.as_str())
            .map(Path::to_path_buf)
            .ok_or_else(|| OrchestratorError::SessionNotFound(session_id.as_str().to_owned()))?;
        let mut lf = self.read_layout_file(&sess_dir)?;
        for b in &mut lf.surfaces {
            if b.id == surface_id.as_str() {
                b.deleted = true;
            }
        }
        atomic_write(&sess_dir.join("layout.json"), &to_json(&lf)?)
    }
}
