//! Per-daemon runtime paths. service-host owns the service manifest (writes/removal/discovery).

use std::path::{Path, PathBuf};

pub fn stopped_sessions_path(dir: &Path) -> PathBuf {
    dir.join("stopped-sessions.txt")
}
