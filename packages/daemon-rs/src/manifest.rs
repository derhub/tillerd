//! Deterministic socket/manifest paths and atomic manifest writes.
//! Paths: `~/.athing/{daemon.json,daemon.sock,hooks.sock}`, honoring `ATHING_DIR`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub fn athing_dir() -> PathBuf {
    match std::env::var_os("ATHING_DIR") {
        Some(v) if !v.is_empty() => {
            let p = PathBuf::from(v);
            // Make relative paths absolute against cwd (mirrors path.resolve()).
            if p.is_absolute() {
                p
            } else {
                std::env::current_dir().unwrap_or_default().join(p)
            }
        }
        _ => home_dir().join(".athing"),
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}

pub fn manifest_path(dir: &Path) -> PathBuf {
    dir.join("daemon.json")
}
pub fn daemon_sock(dir: &Path) -> PathBuf {
    dir.join("daemon.sock")
}
pub fn hooks_sock(dir: &Path) -> PathBuf {
    dir.join("hooks.sock")
}
pub fn stopped_sessions_path(dir: &Path) -> PathBuf {
    dir.join("stopped-sessions.txt")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestData {
    pub pid: u32,
    pub version: String,
}

pub struct Manifest {
    dir: PathBuf,
}

impl Manifest {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn path(&self) -> PathBuf {
        manifest_path(&self.dir)
    }

    pub fn write_for_pid(&self, pid: u32, version: &str) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.path();
        let tmp = path.with_extension("json.tmp");
        let data = ManifestData {
            pid,
            version: version.to_string(),
        };
        fs::write(&tmp, serde_json::to_vec(&data).expect("manifest serialize"))?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn write(&self, version: &str) -> std::io::Result<()> {
        self.write_for_pid(std::process::id(), version)
    }

    pub fn remove(&self) {
        let _ = fs::remove_file(self.path());
    }

    pub fn read(dir: &Path) -> Option<ManifestData> {
        let raw = fs::read(manifest_path(dir)).ok()?;
        serde_json::from_slice(&raw).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_read_roundtrip_and_shape() {
        let dir = std::env::temp_dir().join(format!("athing-manifest-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let m = Manifest::new(dir.clone());
        m.write_for_pid(4321, "9.9.9").unwrap();

        // Exact JSON shape and key order parity with the reference.
        let raw = fs::read_to_string(manifest_path(&dir)).unwrap();
        assert_eq!(raw, r#"{"pid":4321,"version":"9.9.9"}"#);

        let read = Manifest::read(&dir).unwrap();
        assert_eq!(
            read,
            ManifestData {
                pid: 4321,
                version: "9.9.9".into()
            }
        );

        m.remove();
        assert!(Manifest::read(&dir).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn deterministic_paths() {
        let dir = PathBuf::from("/base");
        assert_eq!(manifest_path(&dir), PathBuf::from("/base/daemon.json"));
        assert_eq!(daemon_sock(&dir), PathBuf::from("/base/daemon.sock"));
        assert_eq!(hooks_sock(&dir), PathBuf::from("/base/hooks.sock"));
    }
}
