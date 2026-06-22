use std::collections::HashMap;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
pub use tempfile::TempDir;

use crate::context::Ctx;
use crate::infra::daemon_pty_api::{FakeRuntime, Runtime};
use crate::shared::kv::SqliteKv;

pub async fn make_ctx(dir: &TempDir) -> Ctx {
    let opts = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .shared_cache(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    let kv = SqliteKv::in_memory().await.unwrap();
    Ctx::new(
        pool,
        kv,
        dir.path().to_path_buf(),
        Runtime::Fake(Arc::new(FakeRuntime::new())),
    )
}

pub fn default_keys() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("new-session".to_owned(), "ctrl+n".to_owned());
    m.insert("rename".to_owned(), "F2".to_owned());
    m
}

/// The default keymap serialized to the JSON-string wire form the keybinding
/// command/query DTOs carry (`defaults_json`).
pub fn default_keys_json() -> String {
    serde_json::to_string(&default_keys()).unwrap()
}
