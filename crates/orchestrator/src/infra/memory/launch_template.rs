use super::*;

impl MemoryBackend {
    pub(crate) fn create_launch_template(
        &self,
        draft: NewLaunchTemplate,
    ) -> Result<LaunchTemplate> {
        let template = LaunchTemplate {
            id: LaunchTemplateId::mint(),
            project_id: draft.project_id,
            spec_version: draft.spec_version,
            spec_json: draft.spec_json,
        };
        self.inner
            .lock()
            .unwrap()
            .launch_templates
            .insert(template.id.as_str().to_string(), template.clone());
        Ok(template)
    }

    pub(crate) fn get_launch_template(
        &self,
        id: &LaunchTemplateId,
    ) -> Result<Option<LaunchTemplate>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .launch_templates
            .get(id.as_str())
            .cloned())
    }

    pub(crate) fn set_launch_template_spec(
        &self,
        id: &LaunchTemplateId,
        spec_version: u32,
        spec_json: &str,
    ) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        match inner.launch_templates.get_mut(id.as_str()) {
            Some(t) => {
                t.spec_version = spec_version;
                t.spec_json = spec_json.to_string();
                Ok(())
            }
            None => Err(OrchestratorError::LaunchTemplateNotFound(
                id.as_str().to_string(),
            )),
        }
    }
}
