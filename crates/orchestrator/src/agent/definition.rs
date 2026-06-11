use std::path::Path;

/// Binary-resolution policy for finding the agent executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionPolicy {
    /// Look up the binary on `PATH`.
    PathLookup,
}

/// Static description of an agent: how to launch it, interrupt it, and resolve it.
#[derive(Debug, Clone)]
pub struct AgentDefinition {
    /// Executable name (no path component).
    pub binary: &'static str,
    /// Launch argument template; `{surface_id}` is substituted before spawn.
    pub args_template: &'static [&'static str],
    /// Minimum acceptable CLI version (inclusive), when known.
    pub min_version: Option<&'static str>,
    /// Maximum acceptable CLI version (exclusive), when known.
    pub max_version: Option<&'static str>,
    /// Byte sequence sent to the PTY to interrupt an in-progress turn.
    pub interrupt_seq: &'static [u8],
    /// How to locate the binary on the host system.
    pub resolution: ResolutionPolicy,
}

/// Default agent definition wired to the coding-agent CLI.
pub const AGENT_DEF: AgentDefinition = AgentDefinition {
    binary: "claude",
    args_template: &[
        "--session-id",
        "{surface_id}",
        "--output-format",
        "stream-json",
    ],
    min_version: None,
    max_version: None,
    interrupt_seq: b"\x03",
    resolution: ResolutionPolicy::PathLookup,
};

impl AgentDefinition {
    /// Resolve the binary to a concrete executable path per the resolution policy,
    /// or `None` when it is not found.
    pub fn resolve_binary(&self) -> Option<String> {
        match self.resolution {
            ResolutionPolicy::PathLookup => resolve_on_path(self.binary),
        }
    }

    /// Build the launch arguments with `{surface_id}` substituted.
    pub fn args_for(&self, surface_id: &str) -> Vec<String> {
        self.args_template
            .iter()
            .map(|arg| arg.replace("{surface_id}", surface_id))
            .collect()
    }
}

fn resolve_on_path(binary: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(binary))
        .find(|candidate| is_executable(candidate))
        .map(|p| p.to_string_lossy().into_owned())
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_def_has_surface_id_placeholder() {
        assert!(
            AGENT_DEF
                .args_template
                .iter()
                .any(|a| a.contains("{surface_id}")),
            "args_template must contain {{surface_id}} placeholder"
        );
    }

    #[test]
    fn agent_def_interrupt_seq_is_ctrl_c() {
        assert_eq!(AGENT_DEF.interrupt_seq, b"\x03");
    }

    #[test]
    fn args_for_substitutes_surface_id() {
        assert_eq!(
            AGENT_DEF.args_for("surf-42"),
            vec!["--session-id", "surf-42", "--output-format", "stream-json"],
        );
    }

    #[test]
    fn resolve_binary_finds_path_executable() {
        let def = AgentDefinition {
            binary: "cat",
            ..AGENT_DEF
        };
        assert!(def.resolve_binary().is_some());
    }

    #[test]
    fn resolve_binary_returns_none_when_absent() {
        let def = AgentDefinition {
            binary: "tillerd-no-such-binary-zzz",
            ..AGENT_DEF
        };
        assert!(def.resolve_binary().is_none());
    }
}
