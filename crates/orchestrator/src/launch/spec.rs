use serde::{Deserialize, Serialize};

use crate::error::{OrchestratorError, Result};

pub const CURRENT_SPEC_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaunchSpec {
    pub version: u32,
    pub items: Vec<LaunchItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaunchItem {
    pub target: String,
    pub placement: Option<String>,
    pub command: CommandRef,
    pub worktree: Option<WorktreeStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CommandRef {
    LibraryRef {
        library_ref: String,
    },
    Inline {
        executable: String,
        args: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeStep {
    pub branch: String,
    pub path: String,
}

/// Parse a JSON blob as a `LaunchSpec`, returning a typed error for missing or invalid version.
pub fn parse_spec(blob: &str) -> Result<LaunchSpec> {
    let raw: serde_json::Value = serde_json::from_str(blob)
        .map_err(|e| OrchestratorError::LaunchSpecInvalid(e.to_string()))?;

    match raw.get("version") {
        None | Some(serde_json::Value::Null) => {
            return Err(OrchestratorError::LaunchSpecInvalid(
                "missing version field".to_string(),
            ))
        }
        _ => {}
    }

    let spec: LaunchSpec = serde_json::from_value(raw)
        .map_err(|e| OrchestratorError::LaunchSpecInvalid(e.to_string()))?;

    if spec.version == 0 {
        return Err(OrchestratorError::LaunchSpecInvalid(
            "version must be >= 1".to_string(),
        ));
    }

    Ok(spec)
}

/// Migrate a stored blob at `from_version` to `CURRENT_SPEC_VERSION`.
///
/// Returns `(spec, Some(new_blob))` when migration ran (caller should write back),
/// or `(spec, None)` when `from_version == CURRENT_SPEC_VERSION`.
///
/// Returns `LaunchSpecVersionTooNew` when `from_version > CURRENT_SPEC_VERSION`.
pub fn migrate(blob: &str, from_version: u32) -> Result<(LaunchSpec, Option<String>)> {
    if from_version > CURRENT_SPEC_VERSION {
        return Err(OrchestratorError::LaunchSpecVersionTooNew {
            found: from_version,
            supported: CURRENT_SPEC_VERSION,
        });
    }

    if from_version == CURRENT_SPEC_VERSION {
        let spec = parse_spec(blob)?;
        return Ok((spec, None));
    }

    // Apply incremental migrations from `from_version` up to `CURRENT_SPEC_VERSION`.
    let mut current_blob = blob.to_string();
    let mut current_version = from_version;

    while current_version < CURRENT_SPEC_VERSION {
        current_blob = apply_migration_step(&current_blob, current_version)?;
        current_version += 1;
    }

    let spec = parse_spec(&current_blob)?;
    Ok((spec, Some(current_blob)))
}

/// Apply a single migration step from `step_version` to `step_version + 1`.
/// No steps exist yet (v1 is current); this is the extension point.
fn apply_migration_step(blob: &str, _step_version: u32) -> Result<String> {
    Ok(blob.to_string())
}

impl LaunchSpec {
    pub fn mint_placements(&mut self) {
        for item in &mut self.items {
            if item.placement.is_none() {
                item.placement = Some(uuid::Uuid::new_v4().to_string());
            }
        }
    }

    pub fn ensure_unique_placements(&self) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        for item in &self.items {
            if let Some(placement) = &item.placement {
                if !seen.insert(placement.as_str()) {
                    return Err(OrchestratorError::LaunchSpecInvalid(format!(
                        "duplicate placement: {placement}"
                    )));
                }
            }
        }
        Ok(())
    }
}

pub fn instantiate_for_session(blob: &str) -> Result<String> {
    let mut spec = parse_spec(blob)?;
    spec.mint_placements();
    spec.ensure_unique_placements()?;
    serde_json::to_string(&spec).map_err(|e| OrchestratorError::LaunchSpecInvalid(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v1_blob_with_items() -> &'static str {
        r#"{"version":1,"items":[{"target":"terminal","command":{"library_ref":"login-shell"}}]}"#
    }

    fn v1_blob_empty() -> &'static str {
        r#"{"version":1,"items":[]}"#
    }

    #[test]
    fn parse_well_formed_spec_with_items_is_accepted() {
        let spec = parse_spec(v1_blob_with_items()).unwrap();
        assert_eq!(spec.version, 1);
        assert_eq!(spec.items.len(), 1);
    }

    #[test]
    fn parse_empty_item_list_is_valid() {
        let spec = parse_spec(v1_blob_empty()).unwrap();
        assert_eq!(spec.version, 1);
        assert!(spec.items.is_empty());
    }

    #[test]
    fn parse_missing_version_field_is_rejected() {
        let blob = r#"{"items":[]}"#;
        let err = parse_spec(blob).unwrap_err();
        assert!(matches!(err, OrchestratorError::LaunchSpecInvalid(_)));
    }

    #[test]
    fn parse_null_version_is_rejected() {
        let blob = r#"{"version":null,"items":[]}"#;
        let err = parse_spec(blob).unwrap_err();
        assert!(matches!(err, OrchestratorError::LaunchSpecInvalid(_)));
    }

    #[test]
    fn migrate_current_version_passes_through_without_write_back() {
        let (spec, write_back) = migrate(v1_blob_empty(), 1).unwrap();
        assert_eq!(spec.version, 1);
        assert!(write_back.is_none());
    }

    #[test]
    fn migrate_unknown_future_version_is_refused() {
        let err = migrate(v1_blob_empty(), 99).unwrap_err();
        assert!(matches!(
            err,
            OrchestratorError::LaunchSpecVersionTooNew { .. }
        ));
    }

    #[test]
    fn item_with_library_command_ref_is_accepted() {
        let blob = r#"{"version":1,"items":[{"target":"terminal","command":{"library_ref":"login-shell"}}]}"#;
        let spec = parse_spec(blob).unwrap();
        assert!(matches!(
            spec.items[0].command,
            CommandRef::LibraryRef { .. }
        ));
    }

    #[test]
    fn item_with_inline_command_is_accepted() {
        let blob = r#"{"version":1,"items":[{"target":"terminal","command":{"executable":"/bin/bash","args":[]}}]}"#;
        let spec = parse_spec(blob).unwrap();
        assert!(matches!(spec.items[0].command, CommandRef::Inline { .. }));
    }

    #[test]
    fn mint_placements_assigns_to_items_without_one() {
        let mut spec = parse_spec(v1_blob_with_items()).unwrap();
        assert!(spec.items[0].placement.is_none());
        spec.mint_placements();
        assert!(spec.items[0].placement.is_some());
    }

    #[test]
    fn mint_placements_preserves_an_existing_placement() {
        let blob = r#"{"version":1,"items":[{"target":"terminal","placement":"keep","command":{"library_ref":"s"}}]}"#;
        let mut spec = parse_spec(blob).unwrap();
        spec.mint_placements();
        assert_eq!(spec.items[0].placement.as_deref(), Some("keep"));
    }

    #[test]
    fn mint_placements_assigns_distinct_placements() {
        let blob = r#"{"version":1,"items":[{"target":"terminal","command":{"library_ref":"s"}},{"target":"terminal","command":{"library_ref":"s"}}]}"#;
        let mut spec = parse_spec(blob).unwrap();
        spec.mint_placements();
        assert_ne!(spec.items[0].placement, spec.items[1].placement);
    }

    #[test]
    fn ensure_unique_placements_rejects_duplicates() {
        let blob = r#"{"version":1,"items":[{"target":"terminal","placement":"p","command":{"library_ref":"s"}},{"target":"terminal","placement":"p","command":{"library_ref":"s"}}]}"#;
        let spec = parse_spec(blob).unwrap();
        let err = spec.ensure_unique_placements().unwrap_err();
        assert!(matches!(err, OrchestratorError::LaunchSpecInvalid(_)));
    }

    #[test]
    fn ensure_unique_placements_accepts_distinct() {
        let blob = r#"{"version":1,"items":[{"target":"terminal","placement":"a","command":{"library_ref":"s"}},{"target":"terminal","placement":"b","command":{"library_ref":"s"}}]}"#;
        let spec = parse_spec(blob).unwrap();
        assert!(spec.ensure_unique_placements().is_ok());
    }

    #[test]
    fn instantiate_for_session_mints_a_placement_per_item() {
        let minted = instantiate_for_session(v1_blob_with_items()).unwrap();
        let spec = parse_spec(&minted).unwrap();
        assert!(spec.items.iter().all(|i| i.placement.is_some()));
    }
}
