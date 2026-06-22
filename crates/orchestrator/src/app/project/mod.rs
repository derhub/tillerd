//! Project CQS command/query objects.
//!
//! Each command loads, applies an entity rule, and persists. No transport
//! knowledge; no `Box<dyn Command>`. Queries return data and perform no writes.

mod common;
mod view;

pub mod archive_project;
pub mod discard_project;
pub mod duplicate_project;
pub mod get_project_by_id;
pub mod list_projects_by_workspace;
pub mod move_project;
pub mod new_project_cmd;
pub mod pin_project;
pub mod rename_project;
pub mod reorder_project;
pub mod restore_project;
pub mod search_projects;
pub mod stop_project_surfaces;
pub mod unpin_project;

#[cfg(test)]
pub(crate) mod test_util;

pub use archive_project::ArchiveProject;
pub use discard_project::DiscardProject;
pub use duplicate_project::DuplicateProject;
pub use get_project_by_id::GetProjectById;
pub use list_projects_by_workspace::ListProjectsByWorkspace;
pub use move_project::MoveProject;
pub use new_project_cmd::NewProjectCmd;
pub use pin_project::PinProject;
pub use rename_project::RenameProject;
pub use reorder_project::ReorderProject;
pub use restore_project::RestoreProject;
pub use search_projects::SearchProjects;
pub use stop_project_surfaces::StopProjectSurfaces;
pub use unpin_project::UnpinProject;
pub use view::ProjectView;

/// The unfiled-project id as a primitive, for hosts that default a missing id
/// without reaching the domain newtype.
pub fn unfiled_project_id() -> String {
    crate::entities::ProjectId::unfiled().as_str().to_string()
}
