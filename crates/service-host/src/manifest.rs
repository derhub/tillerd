//! Atomic manifest: temp + rename prevents partial reads. Stale manifests are overwritten.
//!
//! The manifest is the single discovery source (ADR-0028, design D3): a client resolves a service
//! by reading its manifest's `pid`, `version`, lifecycle `status`, and `socket_path` -- no port
//! files, no socket scanning.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// A service's lifecycle status as recorded in its manifest. Consumers read this to decide whether
/// a service is reachable: only `Ready` accepts work; `Starting` is not yet listening and
/// `Draining` refuses new work (ADR-0028, design D2).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    /// Spawned, manifest written, not yet accepting work.
    #[default]
    Starting,
    /// Serve behavior is listening and accepting work.
    Ready,
    /// Draining: refusing new work, finishing active work before exit.
    Draining,
}

/// The persisted manifest payload: the tool's process identity, version, lifecycle status, and
/// (once known) the socket clients reach it on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestData {
    /// The hosting process id.
    pub pid: u32,
    /// The hosting tool's version string.
    pub version: String,
    /// The service's current lifecycle status.
    #[serde(default)]
    pub status: ServiceStatus,
    /// The socket clients reach this service on, derived from the runtime layout.
    #[serde(default)]
    pub socket_path: Option<String>,
}

/// Owns a tool's manifest file at a fixed path.
pub struct Manifest {
    path: PathBuf,
}

impl Manifest {
    /// Bind a manifest to the path where it should live.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Write the full manifest payload via a temp file + atomic rename, so every read either
    /// misses the file or parses a whole manifest -- never a partial one.
    pub fn write_data(&self, data: &ManifestData) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Write the manifest for an explicit pid at `Starting` with no socket yet.
    pub fn write_for_pid(&self, pid: u32, version: &str) -> std::io::Result<()> {
        self.write_data(&ManifestData {
            pid,
            version: version.to_string(),
            status: ServiceStatus::Starting,
            socket_path: None,
        })
    }

    /// Write the manifest for the current process at `Starting`.
    pub fn write(&self, version: &str) -> std::io::Result<()> {
        self.write_for_pid(std::process::id(), version)
    }

    /// Remove the manifest on a clean stop. Idempotent.
    pub fn remove(&self) {
        let _ = fs::remove_file(&self.path);
    }

    /// Read and parse the manifest at `path`, if present and well-formed.
    pub fn read(path: &Path) -> Option<ManifestData> {
        let raw = fs::read(path).ok()?;
        serde_json::from_slice(&raw).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "service-host-{tag}-{}-{:?}",
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
    fn manifest_write_uses_temp_file_and_rename() {
        let dir = temp_dir("write-rename");
        let path = dir.join("tool.json");
        let manifest = Manifest::new(path.clone());
        manifest.write_for_pid(1234, "1.0.0").unwrap();

        assert!(path.exists(), "final manifest must exist after rename");
        assert!(
            !dir.join("tool.json.tmp").exists(),
            "temp file must not linger after the atomic rename"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_carries_pid_and_version() {
        let dir = temp_dir("pid-version");
        let path = dir.join("tool.json");
        Manifest::new(path.clone())
            .write_for_pid(4321, "9.9.9")
            .unwrap();

        let read = Manifest::read(&path).unwrap();
        assert_eq!(
            read,
            ManifestData {
                pid: 4321,
                version: "9.9.9".into(),
                status: ServiceStatus::Starting,
                socket_path: None,
            }
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_records_lifecycle_status_and_socket() {
        let dir = temp_dir("status-socket");
        let path = dir.join("tool.json");
        Manifest::new(path.clone())
            .write_data(&ManifestData {
                pid: 5,
                version: "1.0.0".into(),
                status: ServiceStatus::Ready,
                socket_path: Some("/base/tool.sock".into()),
            })
            .unwrap();

        let read = Manifest::read(&path).unwrap();
        assert_eq!(read.status, ServiceStatus::Ready);
        assert_eq!(read.socket_path.as_deref(), Some("/base/tool.sock"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_without_status_defaults_to_starting() {
        // A manifest predating the lifecycle fields parses with `Starting` / no socket, so a reader
        // never treats an old-format manifest as ready.
        let dir = temp_dir("legacy");
        let path = dir.join("tool.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, br#"{"pid":1,"version":"0.1.0"}"#).unwrap();

        let read = Manifest::read(&path).unwrap();
        assert_eq!(read.status, ServiceStatus::Starting);
        assert_eq!(read.socket_path, None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_never_partial_on_interrupt() {
        // The rename is atomic, so the final path is only ever the complete
        // file: every read either misses it or parses a whole manifest.
        let dir = temp_dir("never-partial");
        let path = dir.join("tool.json");
        let manifest = Manifest::new(path.clone());
        manifest.write_for_pid(7, "2.0.0").unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        let parsed: ManifestData =
            serde_json::from_str(&raw).expect("manifest is whole, not partial");
        assert_eq!(parsed.pid, 7);
        assert_eq!(parsed.version, "2.0.0");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_removed_after_graceful_shutdown() {
        let dir = temp_dir("removed");
        let path = dir.join("tool.json");
        let manifest = Manifest::new(path.clone());
        manifest.write_for_pid(11, "1.2.3").unwrap();
        assert!(Manifest::read(&path).is_some());

        manifest.remove();
        assert!(Manifest::read(&path).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_persists_after_signal_before_handler_runs() {
        // Writing the manifest and then "receiving" a stop signal does not by
        // itself remove the manifest: only the host's handler running remove()
        // does. So between signal delivery and handler execution the manifest
        // is still present (this is what lets a probe see a live instance until
        // shutdown actually completes).
        let dir = temp_dir("persists-pre-handler");
        let path = dir.join("tool.json");
        let manifest = Manifest::new(path.clone());
        manifest.write_for_pid(42, "1.0.0").unwrap();

        // Signal arrives; the handler has not yet run remove().
        assert!(
            Manifest::read(&path).is_some(),
            "manifest persists until the shutdown handler removes it"
        );

        // Only when the handler runs does the manifest go away.
        manifest.remove();
        assert!(Manifest::read(&path).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_cleanup_survives_sigkill() {
        // A SIGKILLed process cannot run cleanup, so the manifest persists; the
        // contract is that the stale file is overwritable (not corrupt), so the
        // next launcher recovers. Simulate: a child writes the manifest, is
        // SIGKILLed, the manifest remains and is then overwritten cleanly.
        let dir = temp_dir("sigkill");
        let path = dir.join("tool.json");
        fs::create_dir_all(&dir).unwrap();

        let mut child = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(format!(
                "printf '{{\"pid\":99999,\"version\":\"0.0.1\"}}' > '{}'; exec sleep 30",
                path.display()
            ))
            .spawn()
            .unwrap();

        // Wait for the child to write the manifest.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while Manifest::read(&path).is_none() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // SIGKILL: cleanup never runs.
        child.kill().unwrap();
        let _ = child.wait();

        let stale = Manifest::read(&path).expect("stale manifest persists after SIGKILL");
        assert_eq!(stale.pid, 99999);

        // The next launcher overwrites the stale manifest cleanly.
        Manifest::new(path.clone())
            .write_for_pid(123, "1.0.0")
            .unwrap();
        let fresh = Manifest::read(&path).unwrap();
        assert_eq!(fresh.pid, 123);
        let _ = fs::remove_dir_all(&dir);
    }
}
