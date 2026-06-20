use super::*;

impl MemoryBackend {
    pub(crate) fn create_surface(&self, draft: NewSurface) -> Result<Surface> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(placement) = &draft.placement {
            let clash = inner.surfaces.values().any(|r| {
                !r.deleted
                    && r.surface.session_id == draft.session_id
                    && r.surface.placement.as_deref() == Some(placement.as_str())
            });
            if clash {
                return Err(OrchestratorError::SurfaceConflict(placement.clone()));
            }
        }
        let surface = Surface {
            id: draft.id.unwrap_or_else(SurfaceId::mint),
            session_id: draft.session_id,
            kind: draft.kind,
            cwd: draft.cwd,
            last_status: None,
            placement: draft.placement,
        };
        inner.surfaces.insert(
            surface.id.as_str().to_string(),
            SurfaceRecord {
                surface: surface.clone(),
                deleted: false,
            },
        );
        Ok(surface)
    }

    pub(crate) fn get_surface(&self, id: &SurfaceId) -> Result<Option<Surface>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .surfaces
            .get(id.as_str())
            .filter(|r| !r.deleted)
            .map(|r| r.surface.clone()))
    }

    pub(crate) fn find_session_surface_by_placement(
        &self,
        session_id: &SessionId,
        placement: &str,
    ) -> Result<Option<Surface>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .surfaces
            .values()
            .filter(|r| {
                !r.deleted
                    && r.surface.session_id.as_str() == session_id.as_str()
                    && r.surface.placement.as_deref() == Some(placement)
            })
            .map(|r| r.surface.clone())
            .next())
    }

    pub(crate) fn list_resumable_surfaces(&self) -> Result<Vec<Surface>> {
        let mut inner = self.inner.lock().unwrap();
        let ids: Vec<String> = inner
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
            .map(|r| r.surface.id.as_str().to_string())
            .collect();
        // Lazy-migrate null-placement rows so resume can bind by placement.
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(r) = inner.surfaces.get_mut(&id) {
                if r.surface.placement.is_none() {
                    r.surface.placement = Some(uuid::Uuid::new_v4().to_string());
                }
                out.push(r.surface.clone());
            }
        }
        Ok(out)
    }

    pub(crate) fn update_surface_status(&self, id: &SurfaceId, status: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(r) = inner.surfaces.get_mut(id.as_str()) {
            r.surface.last_status = Some(status.to_string());
        }
        Ok(())
    }

    pub(crate) fn soft_delete_surface(&self, id: &SurfaceId) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(r) = inner.surfaces.get_mut(id.as_str()) {
            r.deleted = true;
        }
        Ok(())
    }

    pub(crate) fn add_surface_to_session(
        &self,
        session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()> {
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

    pub(crate) fn remove_surface_from_session(
        &self,
        _session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()> {
        self.soft_delete_surface(surface_id)
    }
}
