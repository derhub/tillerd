use std::path::{Path, PathBuf};
use std::sync::Arc;

use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::events::notification::NotificationSink;
use crate::events::surface::SurfaceSink;
use crate::infra::daemon_pty_api::Runtime;
use crate::shared::bus::{Broadcast, Registry};
use crate::shared::kv::SqliteKv;
use crate::shared::Result;

pub type SqliteTx<'c> = Transaction<'c, Sqlite>;

struct CtxInner {
    db: SqlitePool,
    kv: SqliteKv,
    fs_root: PathBuf,
    runtime: Arc<Runtime>,
    notifications_changed: Broadcast<dyn NotificationSink>,
    surface_sinks: Arc<Registry<dyn SurfaceSink>>,
}

#[derive(Clone)]
pub struct Ctx(Arc<CtxInner>);

impl Ctx {
    pub fn new(db: SqlitePool, kv: SqliteKv, fs_root: PathBuf, runtime: Runtime) -> Self {
        Ctx(Arc::new(CtxInner {
            db,
            kv,
            fs_root,
            runtime: Arc::new(runtime),
            notifications_changed: Broadcast::default(),
            surface_sinks: Arc::default(),
        }))
    }

    pub fn db(&self) -> &SqlitePool {
        &self.0.db
    }

    pub fn kv(&self) -> &SqliteKv {
        &self.0.kv
    }

    pub fn fs_root(&self) -> &Path {
        &self.0.fs_root
    }

    pub fn runtime(&self) -> &Runtime {
        &self.0.runtime
    }

    pub fn runtime_arc(&self) -> &Arc<Runtime> {
        &self.0.runtime
    }

    /// The notifications-changed fan-out. The recording layer announces each
    /// persisted notification here; the host subscribes a sink to push it to the
    /// renderer.
    pub fn notifications_changed(&self) -> &Broadcast<dyn NotificationSink> {
        &self.0.notifications_changed
    }

    /// The key-scoped registry of surface sinks. A subscribe command registers a
    /// client sink under a surface id; the pump dispatches each frame to the
    /// sinks for that id. Cloned out as an `Arc` so the pump shares the instance.
    pub fn surface_sinks(&self) -> &Arc<Registry<dyn SurfaceSink>> {
        &self.0.surface_sinks
    }

    /// Opt-in unit of work for a command that spans multiple writes. Begins a
    /// transaction, runs `f`, then commits on `Ok` or explicitly awaits a rollback
    /// on `Err` -- the caller's error is returned unchanged; a rollback failure is
    /// logged, never masking the original error.
    pub async fn transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: AsyncFnOnce(&mut SqliteTx<'_>) -> Result<T>,
    {
        let mut tx = self.db().begin().await?;
        match f(&mut tx).await {
            Ok(value) => {
                tx.commit().await?;
                Ok(value)
            }
            Err(error) => {
                if let Err(rollback) = tx.rollback().await {
                    tracing::error!(error.type = "db.error", %rollback, "transaction rollback failed");
                }
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::daemon_pty_api::{FakeRuntime, Runtime};
    use crate::shared::Error;

    async fn pool() -> SqlitePool {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use std::str::FromStr;

        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .shared_cache(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE counter (id INTEGER PRIMARY KEY, n INTEGER NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    async fn ctx() -> Ctx {
        let pool = pool().await;
        let kv = SqliteKv::in_memory().await.unwrap();
        Ctx::new(
            pool,
            kv,
            PathBuf::from("/tmp/tillerd-test"),
            Runtime::Fake(Arc::new(FakeRuntime::new())),
        )
    }

    async fn rows(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM counter")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn db_exposes_a_usable_pool() {
        let cx = ctx().await;
        sqlx::query("INSERT INTO counter (n) VALUES (1)")
            .execute(cx.db())
            .await
            .unwrap();
        assert_eq!(rows(cx.db()).await, 1);
    }

    #[tokio::test]
    async fn runtime_returns_the_injected_port() {
        let cx = ctx().await;
        let live = cx.runtime().list().await.unwrap();
        assert!(live.is_empty());
    }

    #[tokio::test]
    async fn transaction_commits_on_ok() {
        let cx = ctx().await;
        cx.transaction(async |tx| {
            sqlx::query("INSERT INTO counter (n) VALUES (1)")
                .execute(&mut **tx)
                .await?;
            sqlx::query("INSERT INTO counter (n) VALUES (2)")
                .execute(&mut **tx)
                .await?;
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(rows(cx.db()).await, 2);
    }

    #[tokio::test]
    async fn transaction_rolls_back_on_err() {
        let cx = ctx().await;
        let result: Result<()> = cx
            .transaction(async |tx| {
                sqlx::query("INSERT INTO counter (n) VALUES (1)")
                    .execute(&mut **tx)
                    .await?;
                Err(Error::Validation {
                    field: "n",
                    reason: "boom".to_owned(),
                })
            })
            .await;
        assert!(matches!(result, Err(Error::Validation { .. })));
        assert_eq!(rows(cx.db()).await, 0);
    }
}
