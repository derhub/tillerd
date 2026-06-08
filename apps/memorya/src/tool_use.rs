//! Post-tool capture helpers: skip list and auto-title.

/// Low-value tools whose events are not worth storing.
pub const SKIP_TOOLS: &[&str] = &[
    "ListMcpResourcesTool",
    "SlashCommand",
    "Skill",
    "TodoWrite",
    "AskUserQuestion",
];

/// Whether a post-tool event for `tool_name` should be ignored.
pub fn should_skip(tool_name: &str) -> bool {
    SKIP_TOOLS.contains(&tool_name)
}

/// Derive a concise title from a tool name and its input object. Picks the
/// most identifying field per tool (path, command, pattern), falling back to a
/// short JSON-ish rendering.
pub fn auto_title(tool_name: &str, tool_input: &serde_json::Value) -> String {
    let primary = match tool_name {
        "Read" | "Write" | "Edit" | "NotebookEdit" => field(tool_input, &["file_path", "path"]),
        "Bash" => field(tool_input, &["command"]),
        "Grep" => field(tool_input, &["pattern"]),
        "Glob" => field(tool_input, &["pattern", "glob"]),
        _ => None,
    };
    match primary {
        Some(p) => format!("{tool_name} {}", shorten(&p, 80)),
        None => tool_name.to_string(),
    }
}

fn field(v: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| v.get(*k).and_then(|x| x.as_str()))
        .map(|s| s.to_string())
}

fn shorten(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max).collect();
        format!("{head}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn skips_low_value_tools() {
        assert!(should_skip("TodoWrite"));
        assert!(!should_skip("Read"));
    }

    #[test]
    fn titles_file_tools_by_path() {
        let t = auto_title("Read", &json!({"file_path": "src/auth.rs"}));
        assert_eq!(t, "Read src/auth.rs");
    }

    #[test]
    fn titles_bash_by_command() {
        let t = auto_title("Bash", &json!({"command": "cargo test --lib"}));
        assert_eq!(t, "Bash cargo test --lib");
    }

    #[test]
    fn falls_back_to_tool_name() {
        let t = auto_title("MysteryTool", &json!({"x": 1}));
        assert_eq!(t, "MysteryTool");
    }
}
