//! `mcp.json` config: de-facto `mcpServers` format, loose at the backend level
//! (unknown keys tolerated) and strict at the root.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

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

pub fn config_path() -> PathBuf {
    athing_dir().join("mcp.json")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendKind {
    Stdio,
    Http,
}

// Doc comments here are intentional: schemars emits them as schema.json
// descriptions, which power editor hints for users authoring mcp.json.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct BackendSpec {
    /// Executable for a process backend.
    #[serde(default)]
    pub command: Option<String>,
    /// Arguments passed to the process backend.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment overrides for the process backend.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Endpoint for a remote backend.
    #[serde(default)]
    pub url: Option<String>,
    /// Headers for the remote backend.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Tool names to expose; omit to expose all.
    #[serde(rename = "allowedTools", default)]
    pub allowed_tools: Option<Vec<String>>,
    /// Defer spawning until first call.
    #[serde(default)]
    pub lazy: bool,
    #[serde(flatten)]
    #[schemars(skip)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("backend '{0}' has neither 'command' nor 'url'")]
    MissingTarget(String),
    #[error("backend '{0}' has both 'command' and 'url'; exactly one is allowed")]
    AmbiguousTarget(String),
    #[error("failed to read config: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] serde_json::Error),
}

impl BackendSpec {
    pub fn kind(&self, name: &str) -> Result<BackendKind, ConfigError> {
        match (self.command.is_some(), self.url.is_some()) {
            (true, false) => Ok(BackendKind::Stdio),
            (false, true) => Ok(BackendKind::Http),
            (false, false) => Err(ConfigError::MissingTarget(name.to_string())),
            (true, true) => Err(ConfigError::AmbiguousTarget(name.to_string())),
        }
    }

    pub fn allows_tool(&self, tool: &str) -> bool {
        match &self.allowed_tools {
            Some(list) => list.iter().any(|t| t == tool),
            None => true,
        }
    }

    pub fn unknown_keys(&self) -> impl Iterator<Item = &String> {
        self.extra.keys()
    }
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpConfig {
    /// Optional JSON Schema pointer for editor validation.
    #[serde(rename = "$schema", default, skip_serializing)]
    #[schemars(rename = "$schema")]
    pub schema: Option<String>,
    /// Backend MCP servers keyed by name.
    #[serde(rename = "mcpServers", default)]
    pub servers: HashMap<String, BackendSpec>,
}

impl McpConfig {
    pub fn from_json(s: &str) -> Result<Self, ConfigError> {
        Ok(serde_json::from_str(s)?)
    }

    // Missing file is not an error: start with no backends.
    pub fn load() -> Result<Self, ConfigError> {
        match std::fs::read_to_string(config_path()) {
            Ok(s) => Self::from_json(&s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(ConfigError::Io(e)),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        for (name, spec) in &self.servers {
            spec.kind(name)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_process_backend() {
        let cfg = McpConfig::from_json(
            r#"{"mcpServers":{"fs":{"command":"npx","args":["-y","server"],"env":{"K":"v"}}}}"#,
        )
        .unwrap();
        let fs = &cfg.servers["fs"];
        assert_eq!(fs.kind("fs").unwrap(), BackendKind::Stdio);
        assert_eq!(fs.args, vec!["-y", "server"]);
        assert_eq!(fs.env.get("K").unwrap(), "v");
    }

    #[test]
    fn parses_a_remote_backend() {
        let cfg = McpConfig::from_json(
            r#"{"mcpServers":{"r":{"url":"https://x/mcp","headers":{"Authorization":"Bearer x"}}}}"#,
        )
        .unwrap();
        assert_eq!(cfg.servers["r"].kind("r").unwrap(), BackendKind::Http);
    }

    #[test]
    fn allowlist_filters_tools() {
        let cfg = McpConfig::from_json(
            r#"{"mcpServers":{"gh":{"command":"x","allowedTools":["a","b"]}}}"#,
        )
        .unwrap();
        let gh = &cfg.servers["gh"];
        assert!(gh.allows_tool("a"));
        assert!(!gh.allows_tool("c"));
    }

    #[test]
    fn omitted_allowlist_allows_all() {
        let cfg = McpConfig::from_json(r#"{"mcpServers":{"gh":{"command":"x"}}}"#).unwrap();
        assert!(cfg.servers["gh"].allows_tool("anything"));
    }

    #[test]
    fn lazy_defaults_off() {
        let cfg = McpConfig::from_json(r#"{"mcpServers":{"gh":{"command":"x"}}}"#).unwrap();
        assert!(!cfg.servers["gh"].lazy);
    }

    #[test]
    fn tolerates_unknown_backend_keys() {
        let cfg =
            McpConfig::from_json(r#"{"mcpServers":{"gh":{"command":"x","cursorOnlyField":true}}}"#)
                .unwrap();
        assert!(cfg.servers["gh"]
            .unknown_keys()
            .any(|k| k == "cursorOnlyField"));
    }

    #[test]
    fn rejects_unknown_root_key() {
        let err = McpConfig::from_json(r#"{"mcpServers":{},"bogus":1}"#);
        assert!(err.is_err());
    }

    #[test]
    fn accepts_schema_pointer() {
        let cfg = McpConfig::from_json(r#"{"$schema":"./schema.json","mcpServers":{}}"#).unwrap();
        assert_eq!(cfg.schema.as_deref(), Some("./schema.json"));
    }

    #[test]
    fn missing_target_is_rejected_by_validate() {
        let cfg = McpConfig::from_json(r#"{"mcpServers":{"bad":{"args":[]}}}"#).unwrap();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn default_location_is_under_home() {
        std::env::remove_var("ATHING_DIR");
        assert!(config_path().ends_with(".athing/mcp.json"));
    }
}
