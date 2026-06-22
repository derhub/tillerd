//! File-backed config stores: settings, profiles, themes, and keybindings.
//! All I/O goes through `shared::fs`. No sqlite, no entity types in shared/kv.

pub mod keybinding;
pub mod profile;
pub mod setting;
pub mod theme;

pub use keybinding::KeybindingStore;
pub use profile::ProfileStore;
pub use setting::SettingStore;
pub use theme::ThemeStore;
