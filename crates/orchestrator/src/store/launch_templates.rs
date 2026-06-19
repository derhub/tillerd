//! Launch template store.

use crate::entities::{LaunchTemplate, LaunchTemplateId, NewLaunchTemplate};
use crate::error::Result;
use crate::store::backend::Backend;

/// Operational store for launch templates.
#[derive(Clone)]
pub struct LaunchTemplates {
    backend: Backend,
}

impl LaunchTemplates {
    pub fn new(backend: Backend) -> Self {
        Self { backend }
    }

    pub async fn create(&self, draft: NewLaunchTemplate) -> Result<LaunchTemplate> {
        self.backend.create_launch_template(draft).await
    }

    pub async fn get(&self, id: LaunchTemplateId) -> Result<Option<LaunchTemplate>> {
        self.backend.get_launch_template(id).await
    }

    pub async fn set_spec(
        &self,
        id: LaunchTemplateId,
        spec_version: u32,
        spec_json: String,
    ) -> Result<()> {
        self.backend
            .set_launch_template_spec(id, spec_version, spec_json)
            .await
    }
}
