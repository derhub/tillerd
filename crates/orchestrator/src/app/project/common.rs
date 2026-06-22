use crate::shared::{Error, Result};

/// Derive a project name from a path, falling back to a default.
pub(super) fn infer_name(name: Option<&str>, root_path: Option<&str>) -> String {
    if let Some(name) = name {
        if !name.trim().is_empty() {
            return name.trim().to_owned();
        }
    }
    if let Some(path) = root_path {
        if let Some(last) = std::path::Path::new(path).file_name() {
            return last.to_string_lossy().into_owned();
        }
    }
    "New Project".to_owned()
}

// -- Cursor helpers (relocated from infra/project.rs) --------------------------

/// Cursor format: `"{pinned}:{sort_order}:{id}"`.
pub(super) fn make_cursor(pinned: bool, sort_order: u32, id: &str) -> String {
    format!("{}:{}:{}", pinned as i64, sort_order, id)
}

pub(super) fn parse_cursor(cursor: &str) -> Result<(i64, i64, String)> {
    let mut parts = cursor.splitn(3, ':');
    let pinned: i64 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| invalid_cursor(cursor))?;
    let sort_order: i64 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| invalid_cursor(cursor))?;
    let id = parts
        .next()
        .ok_or_else(|| invalid_cursor(cursor))?
        .to_owned();
    Ok((pinned, sort_order, id))
}

fn invalid_cursor(cursor: &str) -> Error {
    Error::Validation {
        field: "cursor",
        reason: format!("invalid cursor: {cursor}"),
    }
}
