//! Deterministic socket/manifest path names and read-only manifest access.
//! Names live under a base dir resolved by `service-host` (`~/.athing` honoring
//! `ATHING_DIR`). service-host owns manifest writing/removal via `host::run`;
//! this module only derives the names and reads the manifest for the upgrade
//! handoff (where the predecessor learns the successor's pid).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub fn manifest_path(dir: &Path) -> PathBuf {
    dir.join("daemon.json")
}
pub fn daemon_sock(dir: &Path) -> PathBuf {
    dir.join("daemon.sock")
}
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
        let raw = fs::read(manifest_path(dir)).ok()?;
        serde_json::from_slice(&raw).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_paths() {
        let dir = PathBuf::from("/base");
        assert_eq!(manifest_path(&dir), PathBuf::from("/base/daemon.json"));
        assert_eq!(daemon_sock(&dir), PathBuf::from("/base/daemon.sock"));
    }
}
