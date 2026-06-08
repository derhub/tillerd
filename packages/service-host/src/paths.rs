//! Deterministic resource-path resolution for a hosted tool.
//!
//! The base directory honors an `ATHING_DIR` override with parity to the
//! existing TS + Rust behavior: an absolute override passes through, a relative
//! override resolves against the current working directory (mirroring
//! `path.resolve()`), and an absent override falls back to `~/.athing`. Manifest
//! and socket paths are derived from the resolved base by the tool's identity.

use std::path::{Path, PathBuf};

/// Resolved resource paths for a tool, all rooted at a single base directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    base: PathBuf,
    name: String,
}

impl Paths {
    /// Build paths for a tool `name`, resolving the base directory from an
    /// optional override (`ATHING_DIR`-style) or the default `~/.athing`.
    pub fn resolve(name: &str, base_override: Option<&str>) -> Self {
        Self {
            base: resolve_base_dir(base_override),
            name: name.to_string(),
        }
    }

    /// The resolved base directory every other path is rooted at.
    pub fn base_dir(&self) -> &Path {
        &self.base
    }

    /// The manifest path, derived deterministically as `<base>/<name>.json`.
    pub fn manifest_path(&self) -> PathBuf {
        self.base.join(format!("{}.json", self.name))
    }

    /// The socket path, derived deterministically as `<base>/<name>.sock`.
    pub fn socket_path(&self) -> PathBuf {
        self.base.join(format!("{}.sock", self.name))
    }

    /// The liveness-probe socket, derived as `<base>/<name>-health.sock`. Kept
    /// distinct from [`socket_path`](Self::socket_path) so the probe never squats
    /// a tool whose primary socket is its own control/serve plane.
    pub fn health_socket_path(&self) -> PathBuf {
        self.base.join(format!("{}-health.sock", self.name))
    }
}

/// Resolve a base directory honoring an `ATHING_DIR`-style override.
///
/// Parity rule: absolute passes through, relative resolves against cwd, absent
/// falls back to `~/.athing`.
pub fn resolve_base_dir(base_override: Option<&str>) -> PathBuf {
    match base_override.filter(|v| !v.is_empty()) {
        Some(v) => {
            let p = PathBuf::from(v);
            if p.is_absolute() {
                p
            } else {
                std::env::current_dir().unwrap_or_default().join(p)
            }
        }
        None => home_dir().join(".athing"),
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_path_deterministic_from_base_dir() {
        let paths = Paths::resolve("gate", Some("/base"));
        assert_eq!(paths.manifest_path(), PathBuf::from("/base/gate.json"));
    }

    #[test]
    fn socket_path_deterministic_from_base_dir() {
        let paths = Paths::resolve("gate", Some("/base"));
        assert_eq!(paths.socket_path(), PathBuf::from("/base/gate.sock"));
    }

    #[test]
    fn health_socket_distinct_from_primary_socket() {
        let paths = Paths::resolve("daemon", Some("/base"));
        assert_eq!(
            paths.health_socket_path(),
            PathBuf::from("/base/daemon-health.sock")
        );
        assert_ne!(paths.health_socket_path(), paths.socket_path());
    }

    #[test]
    fn all_paths_rooted_at_base_directory() {
        let paths = Paths::resolve("daemon", Some("/srv/athing"));
        assert!(paths.manifest_path().starts_with("/srv/athing"));
        assert!(paths.socket_path().starts_with("/srv/athing"));
        assert_eq!(paths.base_dir(), Path::new("/srv/athing"));
    }

    #[test]
    fn base_dir_override_honored_via_athing_dir_env() {
        let paths = Paths::resolve("daemon", Some("/override/here"));
        assert_eq!(paths.base_dir(), Path::new("/override/here"));
    }

    #[test]
    fn base_dir_override_absolute_path_unchanged() {
        let resolved = resolve_base_dir(Some("/already/absolute"));
        assert_eq!(resolved, PathBuf::from("/already/absolute"));
    }

    #[test]
    fn base_dir_override_relative_path_resolved_against_cwd() {
        let resolved = resolve_base_dir(Some("rel/sub"));
        let expected = std::env::current_dir().unwrap().join("rel/sub");
        assert_eq!(resolved, expected);
        assert!(resolved.is_absolute());
    }
}
