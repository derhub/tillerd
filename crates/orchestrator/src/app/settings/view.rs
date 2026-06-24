//! Flat read DTOs for the config plane (settings, profiles, themes, keybindings).
//!
//! Settings are file-backed (no `setting` SQL table): each View is a plain
//! `Serialize` struct built in the query handler from the loaded config -- there is
//! no SQL row, so no `FromRow`. Each View serializes to the SAME JSON the prior
//! host `*Response` structs produced, so the wire contract is unchanged.

use serde::Serialize;
use serde_json::Value;

use crate::infra::config::keybinding::KeybindingEntry;
use crate::infra::config::profile::Profile;
use crate::infra::config::theme::Theme;

/// One stored setting at a scope. Serializes to `{ "key": ..., "value": <json> }`,
/// matching the host's prior `SettingEntryResponse`: the persisted `value_json`
/// string is parsed back into its JSON value for the wire.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SettingView {
    pub key: String,
    /// Opaque JSON payload; typed as `unknown` in TypeScript to avoid recursive
    /// expansion of `serde_json::Value` at specta export time.
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Unknown))]
    pub value: Value,
}

/// Read model for a profile. Wraps the loaded `Profile` and serializes transparently
/// (same `{ id, name, settings }` JSON the entity produced).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ProfileView(pub Profile);

/// Read model for a theme. Wraps the loaded `Theme` and serializes transparently
/// (same `{ id, name, origin, data_json }` JSON the entity produced).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ThemeView(pub Theme);

/// Read model for a keybinding. Wraps a merged `KeybindingEntry` and serializes
/// transparently (same `{ action, chord }` JSON the entity produced).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct KeybindingView(pub KeybindingEntry);
