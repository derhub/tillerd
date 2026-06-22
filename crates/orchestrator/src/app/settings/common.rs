//! Shared edge-conversion helpers for the settings handlers.

use crate::entities::project::ProjectId;
use crate::entities::setting::SettingScope;
use crate::shared::{Error, Result};

/// Build a [`SettingScope`] from the wire primitives: a scope discriminant
/// (`"global"` / `"project"`) plus an optional project id. Mirrors the mapping the
/// host previously performed at the IPC boundary -- `project` scope requires a
/// `project_id`; an unknown discriminant is rejected.
pub(crate) fn scope_from_parts(scope: &str, project_id: Option<&str>) -> Result<SettingScope> {
    match scope {
        "global" => Ok(SettingScope::Global),
        "project" => project_id
            .map(|p| SettingScope::Project(ProjectId::new(p)))
            .ok_or_else(|| Error::Validation {
                field: "projectId",
                reason: "project scope requires projectId".to_owned(),
            }),
        other => Err(Error::Validation {
            field: "scope",
            reason: format!("unknown setting scope: {other}"),
        }),
    }
}
