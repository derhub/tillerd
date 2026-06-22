use crate::entities::project::NewProject;

pub(super) fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Derive a project name from a path, falling back to a default.
pub(super) fn infer_name(params: &NewProject) -> String {
    if let Some(ref name) = params.name {
        if !name.trim().is_empty() {
            return name.trim().to_owned();
        }
    }
    if let Some(ref path) = params.root_path {
        if let Some(last) = std::path::Path::new(path).file_name() {
            return last.to_string_lossy().into_owned();
        }
    }
    "New Project".to_owned()
}
