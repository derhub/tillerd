//! Manifest: shape mirrors PTY daemon for cross-tool compatibility.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Persisted record naming the live backend process and the version it serves.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestData {
    /// OS process id of the running backend.
    pub pid: u32,
    /// Version string the running backend serves; adoption requires an exact match (R3).
    pub version: String,
}

/// Resolve the base runtime directory honoring `ATHING_DIR` (R7).
///
/// Parity with the existing TypeScript and Rust behavior: a set `ATHING_DIR` is
/// resolved against the current working directory (absolute values pass
/// through); when unset the default is `~/.athing`.
pub fn athing_dir() -> PathBuf {
    match std::env::var_os("ATHING_DIR") {
        Some(v) if !v.is_empty() => {
            let p = PathBuf::from(v);
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

/// Deterministic manifest path under a base directory.
pub fn manifest_path(dir: &Path) -> PathBuf {
    dir.join("daemon.json")
}

/// Deterministic control-socket path under a base directory.
pub fn socket_path(dir: &Path) -> PathBuf {
    dir.join("daemon.sock")
}

/// Read the manifest from a base directory, or `None` if absent or malformed.
pub fn read(dir: &Path) -> Option<ManifestData> {
    let raw = fs::read(manifest_path(dir)).ok()?;
    serde_json::from_slice(&raw).ok()
}

/// Atomically write `{pid, version}` to the manifest under `dir`.
///
/// Writes to a sibling temp file and renames over the target so a reader never
/// observes a partially written manifest.
pub fn write(dir: &Path, pid: u32, version: &str) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = manifest_path(dir);
    let tmp = path.with_extension("json.tmp");
    let data = ManifestData {
        pid,
        version: version.to_string(),
    };
    fs::write(&tmp, serde_json::to_vec(&data).expect("manifest serialize"))?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// Remove the manifest, ignoring a missing file.
pub fn remove(dir: &Path) {
    let _ = fs::remove_file(manifest_path(dir));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "process-launch-manifest-{}-{}-{tag}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn manifest_round_trips_pid_and_version() {
        let dir = temp_dir("roundtrip");
        write(&dir, 4321, "9.9.9").unwrap();

        let read = read(&dir).unwrap();
        assert_eq!(
            read,
            ManifestData {
                pid: 4321,
                version: "9.9.9".into()
            }
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_serializes_pid_before_version_for_cross_tool_parity() {
        let dir = temp_dir("shape");
        write(&dir, 4321, "9.9.9").unwrap();

        let raw = fs::read_to_string(manifest_path(&dir)).unwrap();
        assert_eq!(raw, r#"{"pid":4321,"version":"9.9.9"}"#);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_returns_none_when_manifest_absent() {
        let dir = temp_dir("absent");
        assert!(read(&dir).is_none());
    }

    #[test]
    fn write_leaves_no_temp_file_behind() {
        let dir = temp_dir("notmp");
        write(&dir, 7, "1.0.0").unwrap();
        let tmp = manifest_path(&dir).with_extension("json.tmp");
        assert!(!tmp.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_and_socket_paths_are_deterministic_from_base_dir() {
        let dir = PathBuf::from("/base");
        assert_eq!(manifest_path(&dir), PathBuf::from("/base/daemon.json"));
        assert_eq!(socket_path(&dir), PathBuf::from("/base/daemon.sock"));
    }
}
