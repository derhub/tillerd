//! Per-tool resource paths rooted at the runtime directory resolved by `tillerd-paths`.

use std::path::{Path, PathBuf};

/// Resolved resource paths for a tool, all rooted at a single base directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    base: PathBuf,
    name: String,
}

impl Paths {
    /// Build paths for a tool `name`, resolving the base directory from an
    /// optional override (`TILLERD_DIR`-style) or the default `~/.tillerd`.
    pub fn resolve(name: &str, base_override: Option<&str>) -> Self {
        Self {
            base: tillerd_paths::runtime_dir_or(base_override),
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
    fn all_paths_rooted_at_base_directory() {
        let paths = Paths::resolve("daemon", Some("/srv/tillerd"));
        assert!(paths.manifest_path().starts_with("/srv/tillerd"));
        assert!(paths.socket_path().starts_with("/srv/tillerd"));
        assert_eq!(paths.base_dir(), Path::new("/srv/tillerd"));
    }

    #[test]
    fn base_dir_override_honored() {
        let paths = Paths::resolve("daemon", Some("/override/here"));
        assert_eq!(paths.base_dir(), Path::new("/override/here"));
    }
}
