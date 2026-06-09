//! Diffing: restart only on spawn-affecting fields. Allowlist-gated env keys.
//! fields, and any env var outside the allowlist — are ignored.

use std::collections::BTreeMap;

/// Description of how a managed backend is launched.
///
/// The spawn-affecting fields gate restart decisions; the non-affecting fields
/// (`metadata`, `logging_level`, `observer`) are carried for the caller's own
/// bookkeeping and never force a restart.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpawnSpec {
    /// Executable to launch. Spawn-affecting.
    pub command: String,
    /// Arguments passed to the executable. Spawn-affecting.
    pub args: Vec<String>,
    /// Working directory for the child. Spawn-affecting.
    pub cwd: Option<String>,
    /// Full environment for the child; only keys on the allowlist are compared.
    pub env: BTreeMap<String, String>,

    /// Free-form metadata; never affects the spawned process.
    pub metadata: BTreeMap<String, String>,
    /// Logging verbosity; never affects the spawned process.
    pub logging_level: Option<String>,
    /// Observer/telemetry sink identity; never affects the spawned process.
    pub observer: Option<String>,
}

/// Return `true` when `a` and `b` differ in any spawn-affecting field.
///
/// Only env keys in `env_allowlist` participate in the comparison; all other env
/// entries, and the non-affecting struct fields, are ignored.
pub fn spawn_fields_differ(a: &SpawnSpec, b: &SpawnSpec, env_allowlist: &[&str]) -> bool {
    if a.command != b.command || a.args != b.args || a.cwd != b.cwd {
        return true;
    }
    env_allowlist
        .iter()
        .any(|key| a.env.get(*key) != b.env.get(*key))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> SpawnSpec {
        SpawnSpec {
            command: "athing-daemon".into(),
            args: vec!["--serve".into()],
            cwd: Some("/work".into()),
            env: BTreeMap::from([("ATHING_DIR".to_string(), "/run".to_string())]),
            ..SpawnSpec::default()
        }
    }

    #[test]
    fn spawn_fields_differ_detects_command_change() {
        let a = base();
        let mut b = base();
        b.command = "other-daemon".into();
        assert!(spawn_fields_differ(&a, &b, &["ATHING_DIR"]));
    }

    #[test]
    fn spawn_fields_differ_detects_args_change() {
        let a = base();
        let mut b = base();
        b.args = vec!["--serve".into(), "--verbose".into()];
        assert!(spawn_fields_differ(&a, &b, &["ATHING_DIR"]));
    }

    #[test]
    fn spawn_fields_differ_detects_env_var_change() {
        let a = base();
        let mut b = base();
        b.env.insert("ATHING_DIR".into(), "/elsewhere".into());
        assert!(spawn_fields_differ(&a, &b, &["ATHING_DIR"]));
    }

    #[test]
    fn spawn_fields_differ_ignores_metadata_change() {
        let a = base();
        let mut b = base();
        b.metadata.insert("label".into(), "blue".into());
        assert!(!spawn_fields_differ(&a, &b, &["ATHING_DIR"]));
    }

    #[test]
    fn spawn_fields_differ_ignores_logging_level_change() {
        let a = base();
        let mut b = base();
        b.logging_level = Some("debug".into());
        assert!(!spawn_fields_differ(&a, &b, &["ATHING_DIR"]));
    }

    #[test]
    fn spawn_fields_differ_ignores_observer_field_change() {
        let a = base();
        let mut b = base();
        b.observer = Some("otlp://collector".into());
        assert!(!spawn_fields_differ(&a, &b, &["ATHING_DIR"]));
    }

    #[test]
    fn spawn_fields_differ_ignores_env_var_outside_allowlist() {
        let a = base();
        let mut b = base();
        b.env.insert("LOG_LEVEL".into(), "trace".into());
        assert!(!spawn_fields_differ(&a, &b, &["ATHING_DIR"]));
    }
}
