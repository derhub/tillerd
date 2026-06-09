//! Shell env: spawned commands resolve as in user terminal.

use std::process::Command;

const PROBE_TIMEOUT_SECS: u64 = 5;

const ENV_NAMES: &[&str] = &[
    "PATH",
    "SSH_AUTH_SOCK",
    "HOMEBREW_PREFIX",
    "HOMEBREW_CELLAR",
    "HOMEBREW_REPOSITORY",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
];

fn start_marker(name: &str) -> String {
    format!("__ATHING_ENV_{name}_START__")
}
fn end_marker(name: &str) -> String {
    format!("__ATHING_ENV_{name}_END__")
}

fn build_capture_command() -> String {
    ENV_NAMES
        .iter()
        .map(|name| {
            format!(
                "printf '%s\\n' '{}'; printenv {} || true; printf '%s\\n' '{}'",
                start_marker(name),
                name,
                end_marker(name)
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn extract_value(output: &str, name: &str) -> Option<String> {
    let start_m = start_marker(name);
    let end_m = end_marker(name);
    let start = output.find(&start_m)? + start_m.len();
    let end = output[start..].find(&end_m)? + start;
    let value = output[start..end]
        .trim_start_matches(['\r', '\n'])
        .trim_end_matches(['\r', '\n']);
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn shell_candidates() -> Vec<String> {
    let mut candidates = Vec::new();
    if let Ok(s) = std::env::var("SHELL") {
        if !s.is_empty() {
            candidates.push(s);
        }
    }
    if cfg!(target_os = "macos") {
        candidates.push("/bin/zsh".into());
        candidates.push("/bin/bash".into());
    } else {
        candidates.push("/bin/bash".into());
        candidates.push("/bin/sh".into());
    }
    let mut seen = std::collections::HashSet::new();
    candidates
        .into_iter()
        .filter(|c| seen.insert(c.clone()))
        .collect()
}

fn probe_shell(shell: &str) -> Option<Vec<(String, String)>> {
    let _ = PROBE_TIMEOUT_SECS;
    let out = Command::new(shell)
        .args(["-ilc", &build_capture_command()])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut result = Vec::new();
    for name in ENV_NAMES {
        if let Some(value) = extract_value(&stdout, name) {
            result.push((name.to_string(), value));
        }
    }
    Some(result)
}

pub fn install_login_shell_env() {
    for shell in shell_candidates() {
        let Some(env) = probe_shell(&shell) else {
            continue;
        };
        let path = env
            .iter()
            .find(|(k, _)| k == "PATH")
            .map(|(_, v)| v.clone());
        let Some(path) = path else {
            continue;
        };
        std::env::set_var("PATH", &path);
        for (name, value) in &env {
            if name == "PATH" {
                continue;
            }
            if std::env::var_os(name).is_none() && !value.is_empty() {
                std::env::set_var(name, value);
            }
        }
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_value_between_markers() {
        let out = format!(
            "{}\n/usr/bin:/bin\n{}\n",
            start_marker("PATH"),
            end_marker("PATH")
        );
        assert_eq!(
            extract_value(&out, "PATH").as_deref(),
            Some("/usr/bin:/bin")
        );
    }

    #[test]
    fn extract_missing_is_none() {
        assert_eq!(extract_value("nothing here", "PATH"), None);
    }
}
