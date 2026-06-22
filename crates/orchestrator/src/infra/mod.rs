//! Infrastructure: per-entity async sqlx repositories, the file-based config
//! stores, the surface runtime, and the schema migrations. Each repository is a
//! unit struct of executor-passing functions over `entities`.

pub mod command;
pub mod config;
pub mod daemon_pty_api;
pub mod launch_template;
pub mod migrate;
pub mod notification;
pub mod project;
pub mod session;
pub mod surface_repo;
pub mod workspace;

pub use command::CommandRepo;
pub use config::{KeybindingStore, ProfileStore, SettingStore, ThemeStore};
pub use launch_template::LaunchTemplateRepo;
pub use notification::NotificationRepo;
pub use project::ProjectRepo;
pub use session::SessionRepo;
pub use surface_repo::SurfaceRepo;
pub use workspace::WorkspaceRepo;
