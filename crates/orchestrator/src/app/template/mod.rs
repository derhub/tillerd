//! Template CQS operations.
//!
//! Two surfaces: project-bound launch templates (sqlite via `LaunchTemplateRepo`)
//! and the portable template library (config/fs via `shared::fs`).
//!
//! Commands mutate and return nothing; queries read and return their `Out`.
//! A Prebuilt library template rejects `DiscardTemplate`; only Custom templates
//! can be removed.

mod common;
mod view;

pub mod apply_template_spec;
pub mod discard_launch_template;
pub mod discard_template;
pub mod export_template;
pub mod get_launch_template_by_id;
pub mod get_template_by_id;
pub mod import_template;
pub mod list_launch_templates_by_project;
pub mod list_templates;
pub mod new_launch_template_cmd;
pub mod pin_template;
pub mod unpin_template;

#[cfg(test)]
pub(crate) mod test_util;

pub use apply_template_spec::ApplyTemplateSpec;
pub use discard_launch_template::DiscardLaunchTemplate;
pub use discard_template::DiscardTemplate;
pub use export_template::ExportTemplate;
pub use get_launch_template_by_id::GetLaunchTemplateById;
pub use get_template_by_id::GetTemplateById;
pub use import_template::ImportTemplate;
pub use list_launch_templates_by_project::ListLaunchTemplatesByProject;
pub use list_templates::ListTemplates;
pub use new_launch_template_cmd::NewLaunchTemplateCmd;
pub use pin_template::PinTemplate;
pub use unpin_template::UnpinTemplate;
pub use view::{LaunchTemplateView, TemplateView};
