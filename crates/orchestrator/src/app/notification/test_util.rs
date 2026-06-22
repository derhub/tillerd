use std::path::PathBuf;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::context::Ctx;
use crate::entities::notification::NotificationRecord;
use crate::infra::runtime::FakeRuntime;
use crate::shared::kv::SqliteKv;

pub(crate) async fn test_pool() -> SqlitePool {
    let opts = SqliteConnectOptions::new()
        .filename(":memory:")
        .shared_cache(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("in-memory pool");
    static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("src/infra/migrations");
    MIGRATOR.run(&pool).await.expect("migrations");
    pool
}

pub(crate) async fn test_ctx() -> Ctx {
    let pool = test_pool().await;
    let kv = SqliteKv::in_memory().await.unwrap();
    Ctx::new(
        pool,
        kv,
        PathBuf::from("/tmp/test"),
        Arc::new(FakeRuntime::new()),
    )
}

pub(crate) fn sample(id: &str) -> NotificationRecord {
    NotificationRecord {
        id: id.to_owned(),
        category: "test".to_owned(),
        severity: "info".to_owned(),
        title: None,
        message: "msg".to_owned(),
        detail: None,
        ts: 1_000,
        session_id: None,
        surface_id: None,
        actions_json: None,
        read: false,
        snooze_until: None,
    }
}

pub(crate) fn sample_at(id: &str, ts: i64) -> NotificationRecord {
    NotificationRecord {
        id: id.to_owned(),
        ts,
        ..sample(id)
    }
}
