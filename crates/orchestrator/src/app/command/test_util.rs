use std::path::PathBuf;
use std::sync::Arc;

use crate::context::Ctx;
use crate::infra::daemon_pty_api::{FakeRuntime, Runtime};
use crate::infra::migrate;
use crate::shared::kv::SqliteKv;

pub(crate) async fn ctx() -> Ctx {
    let pool = migrate::open_memory().await.unwrap();
    let kv = SqliteKv::in_memory().await.unwrap();
    Ctx::new(
        pool,
        kv,
        PathBuf::from("/tmp"),
        Runtime::Fake(Arc::new(FakeRuntime::new())),
    )
}
