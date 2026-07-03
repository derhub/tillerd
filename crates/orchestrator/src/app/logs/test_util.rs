use std::str::FromStr;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tempfile::TempDir;

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
