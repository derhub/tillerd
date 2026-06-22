#![cfg(test)]

use std::sync::Arc;
use tempfile::TempDir;

use crate::context::Ctx;
use crate::infra::daemon_pty_api::{FakeRuntime, Runtime};
use crate::infra::migrate;
use crate::shared::kv::SqliteKv;
use crate::shared::Bus;

pub(crate) async fn ctx(dir: &TempDir) -> (Ctx, Bus<Ctx>) {
    let pool = migrate::open_memory().await.unwrap();
    let kv = SqliteKv::in_memory().await.unwrap();
    let cx = Ctx::new(
        pool,
        kv,
        dir.path().to_path_buf(),
        Runtime::Fake(Arc::new(FakeRuntime::new())),
    );
    let bus = Bus::new(cx.clone());
    (cx, bus)
}

pub(crate) const UNFILED: &str = "00000000-0000-0000-0000-000000000000";
