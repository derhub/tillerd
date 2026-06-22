//! Workspace CQS command/query objects (D4). Every operation is transport-agnostic
//! (no Tauri or HTTP types) and dispatched through `Bus<Ctx>`. Commands mutate and
//! return `Result<()>`; queries read and return `Result<Out>`.
//!
//! Key rules from the design:
//! - Single-write commands pass `cx.db()` directly (one statement is atomic).
//! - Multi-repo cascades call `cx.transaction(|tx| ...)` (commit / awaited rollback).
//! - `ArchiveWorkspace` requires every session under it to be idle (no live surfaces).
//! - The Default workspace cannot be archived or discarded.

mod view;

pub mod archive_workspace;
pub mod discard_workspace;
pub mod get_workspace_by_id;
pub mod list_workspaces;
pub mod new_workspace_cmd;
pub mod pin_workspace;
pub mod rename_workspace;
pub mod reorder_workspace;
pub mod restore_workspace;
pub mod stop_workspace_surfaces;
pub mod unpin_workspace;

#[cfg(test)]
pub(crate) mod test_util;

pub use archive_workspace::ArchiveWorkspace;
pub use discard_workspace::DiscardWorkspace;
pub use get_workspace_by_id::GetWorkspaceById;
pub use list_workspaces::ListWorkspaces;
pub use new_workspace_cmd::NewWorkspaceCmd;
pub use pin_workspace::PinWorkspace;
pub use rename_workspace::RenameWorkspace;
pub use reorder_workspace::ReorderWorkspace;
pub use restore_workspace::RestoreWorkspace;
pub use stop_workspace_surfaces::StopWorkspaceSurfaces;
pub use unpin_workspace::UnpinWorkspace;
pub use view::WorkspaceView;

/// The default workspace id as a primitive, for hosts that default a missing id
/// without reaching the domain newtype.
pub fn default_workspace_id() -> String {
    crate::entities::WorkspaceId::default_id()
        .as_str()
        .to_string()
}
