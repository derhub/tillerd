//! Per-entity async stores over a closed `Backend` enum.
//!
//! The composition root selects a `Backend` for each entity and bundles the stores into a
//! `Storage` aggregate. Leaf consumers hold only the concrete stores they call.

pub mod backend;
pub mod commands;
pub mod launch_templates;
pub mod notifications;
pub mod projects;
pub mod sessions;
pub mod settings;
pub mod storage;
pub mod surfaces;
pub mod workspaces;

pub use backend::Backend;
pub use commands::Commands;
pub use launch_templates::LaunchTemplates;
pub use notifications::Notifications;
pub use projects::{ProjectFilter, Projects};
pub use sessions::{SessionFilter, Sessions};
pub use settings::Settings;
pub use storage::Storage;
pub use surfaces::Surfaces;
pub use workspaces::Workspaces;
