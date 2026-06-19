//! Launch template entity: a reusable launch spec bound to a project.

use super::project::ProjectId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LaunchTemplateId(String);

impl LaunchTemplateId {
    pub fn mint() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchTemplate {
    pub id: LaunchTemplateId,
    pub project_id: ProjectId,
    pub spec_version: u32,
    pub spec_json: String,
}

#[derive(Debug, Clone)]
pub struct NewLaunchTemplate {
    pub project_id: ProjectId,
    pub spec_version: u32,
    pub spec_json: String,
}
