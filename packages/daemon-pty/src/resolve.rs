//! Generic command resolution: absolute path as-is; bare name via login-shell PATH;
//! no command defaults to `$SHELL`. Unresolvable named command → `BinaryNotFound`.

use std::process::Command;

#[derive(Debug)]
pub struct BinaryNotFound(pub String);

impl std::fmt::Display for BinaryNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for BinaryNotFound {}

fn login_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

fn login_shell_which(binary: &str) -> Option<String> {
    let shell = login_shell();
    let out = Command::new(&shell)
        .args(["-lc", &format!("which {binary}")])
        .output()
        .ok()?;
    if out.status.success() {
        let resolved = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !resolved.is_empty() {
            return Some(resolved);
        }
    }
    None
}

pub fn resolve_command(command: Option<&str>) -> Result<String, BinaryNotFound> {
    let Some(command) = command.filter(|c| !c.is_empty()) else {
        return Ok(login_shell());
    };
    if command.starts_with('/') {
        return Ok(command.to_string());
    }
    if let Some(found) = login_shell_which(command) {
        return Ok(found);
    }
    Err(BinaryNotFound(format!(
        "Cannot resolve '{command}'. Provide an absolute path or ensure it is on PATH."
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_path_used_directly() {
        assert_eq!(
            resolve_command(Some("/usr/bin/env")).unwrap(),
            "/usr/bin/env"
        );
    }

    #[test]
    fn no_command_defaults_to_shell() {
        let resolved = resolve_command(None).unwrap();
        assert!(resolved.starts_with('/'));
    }

    #[test]
    fn unresolvable_errors() {
        assert!(resolve_command(Some("definitely-not-a-real-binary-xyz")).is_err());
    }
}
