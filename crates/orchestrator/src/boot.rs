use std::path::PathBuf;
use std::sync::Arc;

use crate::context::Ctx;
use crate::infra::migrate;
use crate::infra::runtime::DaemonRuntime;
use crate::shared;
use crate::shared::bus::Bus;
use crate::shared::kv::SqliteKv;

// ── build_bus ─────────────────────────────────────────────────────────────────

/// Configuration for [`build_bus`]. All paths are resolved by the caller; there
/// is no implicit path discovery here.
pub struct Config {
    /// Path to the SQLite domain database (created if absent).
    pub db_path: PathBuf,
    /// Unix socket path for the PTY daemon.
    pub socket: PathBuf,
    /// Root for `shared::fs`-backed user-config (settings, profiles, themes, keybindings).
    pub fs_root: PathBuf,
    /// Directory where rolling `*.log` files are written. Sub-directory `logs/`
    /// is created automatically by the tracing initializer.
    pub log_dir: PathBuf,
    /// Sink that receives PTY output, status, and exit frames from the daemon.
    /// The tauri transport implements this with a per-surface `ipc::Channel`.
    pub sink: Arc<dyn crate::infra::runtime::SurfaceEventSink>,
}

// Keeps the non-blocking log writer's worker thread alive for the process lifetime.
// Box<dyn Any + Send> avoids a direct tracing-appender dep in this crate; the
// concrete type is WorkerGuard from tracing-appender (owned by tillerd-paths).
static LOG_GUARD: std::sync::OnceLock<Box<dyn std::any::Any + Send + Sync>> =
    std::sync::OnceLock::new();

/// Build the transport-agnostic core: open the pool, run migrations, construct
/// [`Ctx`], and return a [`Bus<Ctx>`]. No Tauri wiring here.
///
/// Initializes JSON-lines rolling-file tracing to `cfg.log_dir/logs/` on the
/// first call; subsequent calls are no-ops for tracing (the global subscriber is
/// already set). The log writer guard is held for the process lifetime internally.
pub async fn build_bus(cfg: &Config) -> shared::Result<Bus<Ctx>> {
    let (guard, _root) = tillerd_paths::logging::init_file_tracing(
        "orchestrator",
        env!("CARGO_PKG_VERSION"),
        &cfg.log_dir,
    );
    let _ = LOG_GUARD.set(Box::new(guard));

    std::fs::create_dir_all(&cfg.fs_root).map_err(shared::Error::Io)?;

    let pool = migrate::open_file(&cfg.db_path).await?;
    let kv = SqliteKv::new(pool.clone());
    let runtime = Arc::new(DaemonRuntime::new(cfg.sink.clone(), cfg.socket.clone()));
    let ctx = Ctx::new(pool, kv, cfg.fs_root.clone(), runtime);
    Ok(Bus::new(ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── build_bus ─────────────────────────────────────────────────────────────
    //
    // These tests exercise the Bus<Ctx> contract produced by build_bus without
    // going through the tracing init (which is process-global). The composition
    // in build_bus is trivially thin; each piece is tested independently.

    async fn memory_ctx() -> Ctx {
        use crate::infra::migrate;
        use crate::infra::runtime::FakeRuntime;

        let pool = migrate::open_memory().await.unwrap();
        let kv = SqliteKv::new(pool.clone());
        Ctx::new(
            pool,
            kv,
            PathBuf::from("/tmp/tillerd-boot-test"),
            Arc::new(FakeRuntime::new()),
        )
    }

    struct NoOp;
    impl crate::shared::cqs::Command<Ctx> for NoOp {
        async fn handle(&self, cx: &Ctx) -> crate::shared::Result<()> {
            let _: i64 = sqlx::query_scalar("SELECT 1").fetch_one(cx.db()).await?;
            Ok(())
        }
    }

    struct CountWorkspaces;
    impl crate::shared::cqs::Query<Ctx> for CountWorkspaces {
        type Out = i64;
        async fn handle(&self, cx: &Ctx) -> crate::shared::Result<i64> {
            let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspace")
                .fetch_one(cx.db())
                .await?;
            Ok(n)
        }
    }

    struct NoopSink;
    impl crate::infra::runtime::SurfaceEventSink for NoopSink {
        fn on_bytes(&self, _: &crate::entities::SurfaceId, _: &[u8]) {}
        fn on_status(&self, _: &crate::entities::SurfaceId, _: &str) {}
        fn on_exit(&self, _: &crate::entities::SurfaceId, _: &str) {}
    }

    #[tokio::test]
    async fn bus_execute_reaches_a_migrated_pool() {
        let bus = Bus::new(memory_ctx().await);
        bus.execute(NoOp).await.unwrap();
    }

    #[tokio::test]
    async fn bus_query_returns_seeded_default_workspace() {
        let bus = Bus::new(memory_ctx().await);
        let count = bus.query(CountWorkspaces).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn bus_cx_exposes_the_underlying_pool() {
        let bus = Bus::new(memory_ctx().await);
        let n: i64 = sqlx::query_scalar("SELECT 1")
            .fetch_one(bus.cx().db())
            .await
            .unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn build_bus_opens_a_file_db_and_returns_a_working_bus() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config {
            db_path: dir.path().join("test.db"),
            socket: dir.path().join("daemon.sock"),
            fs_root: dir.path().join("config"),
            log_dir: dir.path().to_owned(),
            sink: Arc::new(NoopSink),
        };

        let bus = build_bus(&cfg).await.unwrap();
        let count = bus.query(CountWorkspaces).await.unwrap();
        assert_eq!(count, 1, "Default workspace seeded after build_bus");
    }
}
