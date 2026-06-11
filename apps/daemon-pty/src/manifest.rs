//! Manifest: read-only here. service-host owns writes/removal. Upgrade handoff.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tillerd_paths::manifest_in;

pub fn stopped_sessions_path(dir: &Path) -> PathBuf {
    dir.join("stopped-sessions.txt")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestData {
    pub pid: u32,
    pub version: String,
}

pub struct Manifest;

impl Manifest {
    pub fn read(dir: &Path) -> Option<ManifestData> {
        let raw = fs::read(manifest_in(dir)).ok()?;
        serde_json::from_slice(&raw).ok()
    }
}
