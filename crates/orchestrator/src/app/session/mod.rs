//! Session CQS command/query objects.
//!
//! Commands mutate and return `Result<()>`; queries read and return `Result<Out>`.
//! No transport knowledge; transport adapters build these values and call
//! `bus.execute`/`bus.query`. A command never re-dispatches through the bus.

mod common;
mod view;

pub mod apply_launch_spec;
pub mod archive_session;
pub mod arrange_panels;
pub mod discard_session;
pub mod duplicate_session;
pub mod get_launch_spec;
pub mod get_panel_tree;
pub mod get_session_by_id;
pub mod launch_session;
pub mod list_all_sessions;
pub mod list_sessions_by_project;
pub mod move_session;
pub mod new_session_cmd;
pub mod pin_session;
pub mod rename_session;
pub mod reorder_session;
pub mod restore_session;
pub mod search_sessions;
pub mod stop_session_surfaces;
pub mod unpin_session;

#[cfg(test)]
pub(crate) mod test_util;

pub use apply_launch_spec::ApplyLaunchSpec;
pub use archive_session::ArchiveSession;
pub use arrange_panels::ArrangePanels;
pub use discard_session::DiscardSession;
pub use duplicate_session::DuplicateSession;
pub use get_launch_spec::GetLaunchSpec;
pub use get_panel_tree::GetPanelTree;
pub use get_session_by_id::GetSessionById;
pub use launch_session::LaunchSession;
pub use list_all_sessions::ListAllSessions;
pub use list_sessions_by_project::ListSessionsByProject;
pub use move_session::MoveSession;
pub use new_session_cmd::NewSessionCmd;
pub use pin_session::PinSession;
pub use rename_session::RenameSession;
pub use reorder_session::ReorderSession;
pub use restore_session::RestoreSession;
pub use search_sessions::SearchSessions;
pub use stop_session_surfaces::StopSessionSurfaces;
pub use unpin_session::UnpinSession;
pub use view::{LaunchSpecView, SessionView};
