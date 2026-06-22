//! Launch template entity: a reusable launch spec bound to a project.

use serde::{Deserialize, Serialize};

use super::project::ProjectId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct LaunchTemplateId(String);

impl LaunchTemplateId {
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct LaunchTemplate {
    pub id: LaunchTemplateId,
    pub project_id: ProjectId,
    pub spec_version: u32,
    pub spec_json: String,
}
