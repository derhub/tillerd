//! tauri-specta has no object-param command mode, so this LanguageExt wrapper post-processes the
//! generated TS, rewriting positional params into one object param. The wire stays flat:
//! `invoke("cmd", args)` where `args` is the same `{ field: .. }` object.

use std::path::Path;

use tauri_specta::{BuilderConfiguration, LanguageExt};

pub struct ObjectParamTs(pub specta_typescript::Typescript);

impl LanguageExt for ObjectParamTs {
    type Error = specta_typescript::Error;

    fn export(self, cfg: &BuilderConfiguration, path: &Path) -> Result<(), Self::Error> {
        self.0.export(cfg, path)?;
        let src = std::fs::read_to_string(path)?;
        std::fs::write(path, to_object_params(&src))?;
        Ok(())
    }
}

fn to_object_params(src: &str) -> String {
    let mut params = String::with_capacity(src.len() + 256);
    for line in src.lines() {
        params.push_str(&rewrite_line(line));
        params.push('\n');
    }
    // A multiline inline return type carries its `__TAURI_INVOKE("cmd", { .. })` on a trailing line,
    // so payloads are rewritten over the joined source, not per line.
    replace_invoke_payloads(&params)
}

fn rewrite_line(line: &str) -> String {
    let Some(rest) = line.strip_prefix('\t') else {
        return line.to_string();
    };
    let Some(colon) = rest.find(": (") else {
        return line.to_string();
    };
    let name = &rest[..colon];
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return line.to_string();
    }
    let after = &rest[colon + 3..]; // inside the param list, past the opening `(`
    let Some(close) = matching_closer(after, '(', ')', 1) else {
        return line.to_string();
    };
    let params = &after[..close];
    let body = &after[close + 1..];
    if params.trim().is_empty() {
        return line.to_string(); // 0-arg command: leave as-is
    }
    let fields = split_top(params)
        .into_iter()
        .map(|p| match p.find(':') {
            Some(i) => format!("{}: {}", p[..i].trim(), p[i + 1..].trim()),
            None => p,
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!("\t{}: (args: {{ {} }}){}", name, fields, body)
}

// First close at depth 0. `initial_depth` is 1 when `s` starts inside an open delimiter, else 0.
fn matching_closer(s: &str, open: char, close: char, initial_depth: i32) -> Option<usize> {
    let mut depth = initial_depth;
    for (i, ch) in s.char_indices() {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

// depth-aware comma split (keeps `string | null`, `Channel<Vec<u8>>` intact).
fn split_top(s: &str) -> Vec<String> {
    let (mut depth, mut cur, mut out) = (0i32, String::new(), Vec::new());
    for ch in s.chars() {
        match ch {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            _ => {}
        }
        if ch == ',' && depth == 0 {
            out.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(ch);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

// Rewrite each invoke payload object (the `{ .. }` after `__TAURI_INVOKE("cmd",`) to `args`.
fn replace_invoke_payloads(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(pos) = rest.find("\", {") {
        let obj = &rest[pos + 3..]; // slice starting at the payload's `{`
        let Some(end) = matching_closer(obj, '{', '}', 0) else {
            break; // malformed: leave the remainder untouched
        };
        out.push_str(&rest[..pos]);
        out.push_str("\", args");
        rest = &obj[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::to_object_params;

    #[test]
    fn single_line_command_becomes_object_param() {
        let src =
            "\tsessionRename: (id: string, title: string) => typedError<null, string>(__TAURI_INVOKE(\"session_rename\", { id, title })),\n";
        assert_eq!(
            to_object_params(src),
            "\tsessionRename: (args: { id: string; title: string }) => typedError<null, string>(__TAURI_INVOKE(\"session_rename\", args)),\n"
        );
    }

    #[test]
    fn multiline_return_type_payload_is_rewritten() {
        // Payload on a trailing line, past a multiline inline return type.
        let src = "\tsessionGet: (id: string) => typedError<{\n\tid: string,\n} | null, string>(__TAURI_INVOKE(\"session_get\", { id })),\n";
        let out = to_object_params(src);
        assert!(
            out.contains("sessionGet: (args: { id: string }) =>"),
            "param rewritten: {out}"
        );
        assert!(
            out.contains("__TAURI_INVOKE(\"session_get\", args)"),
            "payload rewritten: {out}"
        );
        assert!(!out.contains("{ id })"), "no stale destructure left: {out}");
    }

    #[test]
    fn zero_arg_command_is_untouched() {
        let src = "\tworkspaceList: () => typedError<WorkspaceView[], string>(__TAURI_INVOKE(\"workspace_list\")),\n";
        assert_eq!(to_object_params(src), src);
    }

    #[test]
    fn non_command_lines_pass_through() {
        let src = "export type SessionView = {\n\tid: string,\n};\n";
        assert_eq!(to_object_params(src), src);
    }
}
