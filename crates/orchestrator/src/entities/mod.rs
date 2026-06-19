//! Domain entity types: plain data with value-object ids and enums. Pure — no infra
//! dependencies. The lowest layer; `infra` and `store` build on it.

pub mod command;
pub mod launch_template;
pub mod notification;
pub mod project;
pub mod session;
pub mod setting;
pub mod surface;
pub mod workspace;

pub use command::{Command, CommandId, CommandOrigin, NewCommand};
pub use launch_template::{LaunchTemplate, LaunchTemplateId, NewLaunchTemplate};
pub use notification::NotificationRecord;
pub use project::{NewProject, Project, ProjectId, SourceKind};
pub use session::{NewSession, Session, SessionId, TitleSource};
pub use setting::{SettingEntry, SettingScope};
pub use surface::{NewSurface, Surface, SurfaceId, SurfaceKind};
pub use workspace::{NewWorkspace, Workspace, WorkspaceId};
