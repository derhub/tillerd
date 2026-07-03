use std::collections::HashMap;

use serde::Serialize;

/// Flat read model for a command-library row. Serializes to the SDK `Command` wire
/// shape (the same camelCase JSON the prior `CommandResponse` host struct produced).
///
/// `args`/`env` are stored as JSON text columns (`args_json`/`env_json`); the
/// `#[sqlx(rename = ..., json)]` decodes them straight into the built-in collections,
/// mirroring the `Command` entity's `#[sqlx(json)]` fields.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CommandView {
    pub id: String,
    pub name: String,
    pub origin: String,
    pub cli: String,
    #[sqlx(rename = "args_json", json)]
    pub args: Vec<String>,
    #[sqlx(rename = "env_json", json)]
    pub env: HashMap<String, String>,
}
