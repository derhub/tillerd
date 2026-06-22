use std::path::PathBuf;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::app::notification::list_notifications::ListNotifications;
use crate::app::notification::record_notification::RecordNotification;
use crate::context::Ctx;
use crate::infra::daemon_pty_api::{FakeRuntime, Runtime};
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
        Runtime::Fake(Arc::new(FakeRuntime::new())),
    )
}

/// A `RecordNotification` command for a sample unread notification at `ts = 1_000`.
pub(crate) fn record_cmd(id: &str) -> RecordNotification {
    RecordNotification {
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

/// A `RecordNotification` command for a sample notification at the given `ts`.
pub(crate) fn record_cmd_at(id: &str, ts: i64) -> RecordNotification {
    RecordNotification {
        ts,
        ..record_cmd(id)
    }
}

/// `ListNotifications` over the full (unbounded) feed.
pub(crate) fn list_all() -> ListNotifications {
    ListNotifications {
        limit: None,
        offset: None,
        after: None,
    }
}

/// `ListUnreadNotifications` over the full (unbounded) feed.
pub(crate) fn list_unread_all(
) -> crate::app::notification::list_unread_notifications::ListUnreadNotifications {
    crate::app::notification::list_unread_notifications::ListUnreadNotifications {
        limit: None,
        offset: None,
        after: None,
    }
}
