//! CQS command/query objects for the config plane: settings, profiles, themes,
//! and keybindings. All persistence is file-based via `shared::fs`; no sqlite.
//!
//! Stores are constructed on-demand from `cx.fs_root()` -- they hold no internal
//! cache and are cheap to build.
//!
//! Command-query separation is strict: commands mutate and return `()`, queries
//! read and perform no write.

mod apply_setting;
mod get_setting;
mod list_settings;
mod reload_config;
mod reset_setting;
mod resolve_setting;
mod resolve_settings;

mod activate_profile;
mod discard_profile;
mod duplicate_profile;
mod export_profile;
mod get_active_profile;
mod import_profile;
mod list_profiles;
mod new_profile;
mod rename_profile;

mod activate_theme;
mod discard_theme;
mod export_theme;
mod get_active_theme;
mod import_theme;
mod list_themes;

mod list_keybindings;
mod rebind_key;
mod reset_keybinding;
mod reset_keybindings;
mod resolve_keybinding;

mod common;
mod view;

#[cfg(test)]
pub(crate) mod test_util;

pub use activate_profile::ActivateProfile;
pub use activate_theme::ActivateTheme;
pub use apply_setting::ApplySetting;
pub use discard_profile::DiscardProfile;
pub use discard_theme::DiscardTheme;
pub use duplicate_profile::DuplicateProfile;
pub use export_profile::ExportProfile;
pub use export_theme::ExportTheme;
pub use get_active_profile::GetActiveProfile;
pub use get_active_theme::GetActiveTheme;
pub use get_setting::GetSetting;
pub use import_profile::ImportProfile;
pub use import_theme::ImportTheme;
pub use list_keybindings::ListKeybindings;
pub use list_profiles::ListProfiles;
pub use list_settings::ListSettings;
pub use list_themes::ListThemes;
pub use new_profile::NewProfile;
pub use rebind_key::RebindKey;
pub use reload_config::ReloadConfig;
pub use rename_profile::RenameProfile;
pub use reset_keybinding::ResetKeybinding;
pub use reset_keybindings::ResetKeybindings;
pub use reset_setting::ResetSetting;
pub use resolve_keybinding::ResolveKeybinding;
pub use resolve_setting::ResolveSetting;
pub use resolve_settings::ResolveSettings;
pub use view::{KeybindingView, ProfileView, SettingView, ThemeView};
