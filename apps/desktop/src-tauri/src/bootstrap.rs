use std::path::PathBuf;
use std::process::Command;

use serde::Serialize;

use tillerd_paths::{resolve_notify_bin, runtime_dir};

/// Startup values the host resolves for the renderer (design D4): the agent binary, its version,
/// the prepared hook command, and the runtime directory the engine injects so the agent's hook
/// client derives the gate socket. The renderer gates the version against the adapter's supported
/// range (§5.5) and constructs the ports.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    path: String,
    version: String,
    hook_command: Option<String>,
    tillerd_dir: String,
    agent_home: String,
    home_dir: String,
}

#[tauri::command]
pub fn agent_bootstrap() -> Result<AgentInfo, String> {
    let path = resolve_agent_binary("claude")?;
    let output = Command::new(&path)
        .arg("--version")
        .output()
        .map_err(|e| format!("run {} --version: {e}", path.display()))?;
    let version = parse_version(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| "could not parse claude --version".to_string())?;
    let hook_command = resolve_notify_bin().map(|p| p.to_string_lossy().into_owned());
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let agent_home = home.join(".claude").to_string_lossy().into_owned();
    Ok(AgentInfo {
        path: path.to_string_lossy().into_owned(),
        version,
        hook_command,
        tillerd_dir: runtime_dir().to_string_lossy().into_owned(),
        agent_home,
        home_dir: home.to_string_lossy().into_owned(),
    })
}

/// Mirror the adapter's `resolveAgentBinary` policy (packages/adapter-claude-code/src/resolve.ts):
/// `CLAUDE_CODE_EXECUTABLE` override, then a login-shell `which` (a GUI-launched app has a sparse
/// `PATH`, so a plain `PATH` scan would miss `claude`), then common install locations.
fn resolve_agent_binary(command: &str) -> Result<PathBuf, String> {
    if let Ok(over) = std::env::var("CLAUDE_CODE_EXECUTABLE") {
        if !over.is_empty() {
            return Ok(PathBuf::from(over));
        }
    }
    if let Some(found) = login_shell_which(command) {
        return Ok(found);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let common = [
        PathBuf::from("/usr/local/bin/claude"),
        PathBuf::from("/usr/bin/claude"),
        PathBuf::from(format!("{home}/.local/bin/claude")),
        PathBuf::from(format!("{home}/.npm-global/bin/claude")),
    ];
    for loc in common {
        if loc.is_file() {
            return Ok(loc);
        }
    }
    Err(format!(
        "cannot resolve '{command}': set CLAUDE_CODE_EXECUTABLE or ensure it is on PATH"
    ))
}

fn login_shell_which(binary: &str) -> Option<PathBuf> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let output = Command::new(shell)
        .arg("-lc")
        .arg(format!("which {binary}"))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if resolved.is_empty() {
        None
    } else {
        Some(PathBuf::from(resolved))
    }
}

/// Pull the first `x.y.z` triple out of a `--version` line.
fn parse_version(out: &str) -> Option<String> {
    let bytes = out.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            let mut dots = 0;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                if bytes[i] == b'.' {
                    dots += 1;
                }
                i += 1;
            }
            if dots >= 2 {
                return Some(out[start..i].trim_end_matches('.').to_string());
            }
        } else {
            i += 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse_version;

    #[test]
    fn parses_version_from_annotated_line() {
        assert_eq!(parse_version("1.2.3 (agent cli)").as_deref(), Some("1.2.3"));
    }

    #[test]
    fn parses_version_with_multi_digit_segments() {
        assert_eq!(parse_version("tool 0.10.42").as_deref(), Some("0.10.42"));
    }

    #[test]
    fn returns_none_when_no_version_present() {
        assert_eq!(parse_version("no version here"), None);
    }

    #[test]
    fn trailing_dot_is_trimmed() {
        assert_eq!(parse_version("1.2.3.").as_deref(), Some("1.2.3"));
    }

    #[test]
    fn two_segment_version_is_none() {
        assert_eq!(parse_version("version 1.2"), None);
    }

    #[test]
    fn picks_first_triple_when_multiple_present() {
        assert_eq!(
            parse_version("tool 1.2.3 (built 4.5.6)").as_deref(),
            Some("1.2.3")
        );
    }

    #[test]
    fn empty_string_is_none() {
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn leading_v_prefix_is_handled() {
        assert_eq!(parse_version("v1.0.0").as_deref(), Some("1.0.0"));
    }
}
