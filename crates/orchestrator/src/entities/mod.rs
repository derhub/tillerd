//! Domain entity types: plain data with value-object ids and enums. Pure -- no infra
//! dependencies. The lowest layer; `infra` and `store` build on it.

pub mod command;
pub mod launch_spec;
pub mod launch_template;
pub mod notification;
pub mod project;
pub mod session;
pub mod setting;
pub mod state_model;
pub mod surface;
pub mod template;
pub mod workspace;

pub use launch_template::{LaunchTemplate, LaunchTemplateId};
pub use project::ProjectId;
pub use surface::{Surface, SurfaceId, SurfaceKind, SurfaceStatus};
pub use workspace::WorkspaceId;
