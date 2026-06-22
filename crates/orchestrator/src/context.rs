//! The orchestrator context: the real resources every operation runs against —
//! the `SqlitePool`, the `SqliteKv`, the user-config root, and the surface runtime
//! port. `Ctx` is cheap to clone and `Send + Sync`, so it survives `.await` and
//! Tauri's `manage`. It holds no pre-built repo aggregate: repos take whatever
//! executor they are handed (`cx.db()` or `&mut *tx`), so nothing is bound to a
//! single connection.
//!
//! Queries and single-statement commands use `db()` directly; a command that spans
//! multiple writes opts into `transaction(|tx| …)`, which commits on `Ok` and
//! explicitly, awaited-rolls-back on `Err` (sqlx rolls back on `Drop`, but `Drop`
//! cannot `.await` and reports no failure — the helper gives deterministic timing
//! and a loggable result).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::infra::runtime::SurfaceRuntime;
use crate::shared::kv::SqliteKv;
use crate::shared::Result;

/// A sqlite transaction over the orchestrator pool, handed to a
/// [`Ctx::transaction`] closure.
pub type SqliteTx<'c> = Transaction<'c, Sqlite>;

/// The resources an operation runs against. Cloning shares the underlying pool and
/// runtime (`Arc`); the config root is a cheap `PathBuf` clone.
#[derive(Clone)]
pub struct Ctx {
    db: SqlitePool,
    kv: Arc<SqliteKv>,
    fs_root: PathBuf,
    runtime: Arc<dyn SurfaceRuntime>,
}

impl Ctx {
    /// Build a context over its resources.
    pub fn new(
        db: SqlitePool,
        kv: SqliteKv,
        fs_root: PathBuf,
        runtime: Arc<dyn SurfaceRuntime>,
    ) -> Self {
        Ctx {
            db,
            kv: Arc::new(kv),
            fs_root,
            runtime,
        }
    }

    /// The connection pool, for queries and single-statement commands.
    pub fn db(&self) -> &SqlitePool {
        &self.db
    }

    /// The key-value store.
    pub fn kv(&self) -> &SqliteKv {
        &self.kv
    }

    /// The user-config root for `shared::fs`-backed config.
    pub fn fs_root(&self) -> &Path {
        &self.fs_root
    }

    /// The surface runtime port.
    pub fn runtime(&self) -> &dyn SurfaceRuntime {
        &*self.runtime
    }

    /// Opt-in unit of work for a command that spans multiple writes. Begins a
    /// transaction, runs `f`, then commits on `Ok` or explicitly awaits a rollback
    /// on `Err` — the caller's error is returned unchanged; a rollback failure is
    /// logged, never masking the original error.
    pub async fn transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: AsyncFnOnce(&mut SqliteTx<'_>) -> Result<T>,
    {
        let mut tx = self.db.begin().await?;
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
    use crate::infra::runtime::FakeRuntime;
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
            Arc::new(FakeRuntime::new()),
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
