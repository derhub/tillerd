//! Daemon manifest (`mcp-gateway.json`): how to reach a running daemon.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::athing_dir;

pub const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub pid: u32,
    pub port: u16,
    pub token: String,
    pub version: String,
}

pub fn manifest_path() -> PathBuf {
    athing_dir().join("mcp-gateway.json")
}

impl Manifest {
    pub fn write(&self) -> std::io::Result<()> {
        let dir = athing_dir();
        std::fs::create_dir_all(&dir)?;
        let path = manifest_path();
        let tmp = dir.join(format!("mcp-gateway.json.tmp.{}", self.pid));
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn read() -> Option<Self> {
        let raw = std::fs::read_to_string(manifest_path()).ok()?;
        serde_json::from_str(&raw).ok()
    }

    pub fn remove() {
        let _ = std::fs::remove_file(manifest_path());
    }

    pub fn is_reusable(&self) -> bool {
        self.version == DAEMON_VERSION && pid_alive(self.pid)
    }
}

/// Best-effort liveness check for a pid (signal 0: exists vs ESRCH).
pub fn pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok()
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}
