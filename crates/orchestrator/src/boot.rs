use std::path::PathBuf;
use std::sync::Arc;

use crate::app::logs::LogFollower;
use crate::app::notification::NotificationSink;
use crate::app::surface::SurfaceStream;
use crate::context::Ctx;
use crate::infra::daemon_pty_api::{DaemonPtyApi, FakeRuntime, Runtime, RuntimeCall};
use crate::infra::migrate;
use crate::shared;
use crate::shared::bus::Bus;
use crate::shared::kv::SqliteKv;

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
    /// Notifications-changed sink. The host subscribes to push each recorded
    /// notification to the renderer; the recording layer announces them here.
    pub notification_sink: Arc<dyn NotificationSink>,
}

// Keeps the non-blocking log writer's worker thread alive for the process lifetime.
// Box<dyn Any + Send> avoids a direct tracing-appender dep in this crate; the
// concrete type is WorkerGuard from tracing-appender (owned by tillerd-paths).
static LOG_GUARD: std::sync::OnceLock<Box<dyn std::any::Any + Send + Sync>> =
    std::sync::OnceLock::new();

/// Install the process-global file-tracing subscriber on first boot.
///
/// Gated to non-test builds: `init_file_tracing` calls `set_global_default`,
/// which mutates the process-wide `tracing` `MAX_LEVEL` static. In the `--lib`
/// unit-test binary that global write races, across threads, with the
/// thread-local `set_default` guards the span-counting tests install; a parallel
/// guard lowering `MAX_LEVEL` in the window where a measured `info_span!` reads
/// it makes that span never get created, so its layer counts zero. Skipping the
/// global install under `cfg(test)` leaves `MAX_LEVEL` governed solely by those
/// thread-local guards (always TRACE while a guard is alive), so every span
/// test's measured dispatch is observed deterministically regardless of order or
/// parallelism.
#[cfg(not(test))]
fn init_tracing(log_dir: &std::path::Path) {
    let (guard, _root) =
        tillerd_paths::logging::init_file_tracing("orchestrator", env!("CARGO_PKG_VERSION"), log_dir);
    let _ = LOG_GUARD.set(Box::new(guard));
}

#[cfg(test)]
fn init_tracing(_log_dir: &std::path::Path) {
    // No-op in the unit-test binary: see the `cfg(not(test))` variant above.
    let _ = &LOG_GUARD;
}

/// Build the transport-agnostic core: open the pool, run migrations, construct
/// [`Ctx`], and return a [`Bus<Ctx>`]. No Tauri wiring here.
///
/// Initializes JSON-lines rolling-file tracing to `cfg.log_dir/logs/` on the
/// first call; subsequent calls are no-ops for tracing (the global subscriber is
/// already set). The log writer guard is held for the process lifetime internally.
pub async fn build_bus(cfg: &Config) -> shared::Result<Bus<Ctx>> {
    init_tracing(&cfg.log_dir);

    std::fs::create_dir_all(&cfg.fs_root).map_err(shared::Error::Io)?;

    let pool = migrate::open_file(&cfg.db_path).await?;
    let kv = SqliteKv::new(pool.clone());

    let runtime = Runtime::Daemon(DaemonPtyApi::new(cfg.socket.clone()));
    let ctx = Ctx::new(pool, kv, cfg.fs_root.clone(), runtime);

    // The middleware stack order lives in `crate::middleware::pipeline`; here is
    // where its recording-layer dependency is wired: the store and this change
    // sink come from `Ctx`, supplied to dispatch rather than hardcoded.
    ctx.notifications_changed()
        .subscribe(cfg.notification_sink.clone());
    // `Ctx` wraps the runtime in `Arc` internally; clone out the same `Arc` so
    // the pump shares the same instance without an extra allocation.
    let runtime_arc = Arc::clone(ctx.runtime_arc());
    // The pump dispatches per-surface through the same registry a subscribe
    // command registers client sinks into.
    let registry = Arc::clone(ctx.surface_sinks());

    tokio::spawn(
        SurfaceStream {
            runtime: runtime_arc,
            registry,
        }
        .run(),
    );

    // Follow the runtime logs directory; appended lines fan out per service to the
    // same registry a `SubscribeLogs` command registers client sinks into.
    let logs_dir = tillerd_paths::logging::logs_dir_in(&cfg.log_dir);
    tokio::spawn(LogFollower::new(logs_dir, Arc::clone(ctx.log_sinks())).run());

    Ok(Bus::new(ctx))
}

/// Build a `:memory:` [`Ctx`] for tests: open an in-memory pool with migrations
/// applied, wire a [`SqliteKv`] over it, and inject a `FakeRuntime` (no daemon, no
/// PTY). This is the app-owned test edge the desktop host's IPC contract test drives
/// every command over -- so the host never reaches into `infra::migrate` /
/// `infra::daemon_pty_api` itself.
pub async fn test_ctx() -> shared::Result<Ctx> {
    Ok(test_ctx_with_probe().await?.0)
}

/// An app-owned probe over a test `Ctx`'s in-memory runtime. Exposes the off-bus
/// runtime writes (`input`/`resize`) a host command shim performs as primitives,
/// so a host-side test asserts them without naming an infra/entities type.
pub struct TestRuntimeProbe(Arc<FakeRuntime>);

impl TestRuntimeProbe {
    /// Recorded `input` writes as `(surface_id, bytes)`, in call order.
    pub fn inputs(&self) -> Vec<(String, Vec<u8>)> {
        self.0
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                RuntimeCall::Input { surface, bytes } => Some((surface.as_str().to_owned(), bytes)),
                _ => None,
            })
            .collect()
    }

    /// Recorded `resize` writes as `(surface_id, cols, rows)`, in call order.
    pub fn resizes(&self) -> Vec<(String, u16, u16)> {
        self.0
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                RuntimeCall::Resize {
                    surface,
                    cols,
                    rows,
                } => Some((surface.as_str().to_owned(), cols, rows)),
                _ => None,
            })
            .collect()
    }

    /// Recorded `spawn` surface ids, in call order. A revisit re-attaches an
    /// existing surface, so a second open at the same placement adds no spawn.
    pub fn spawns(&self) -> Vec<String> {
        self.0
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                RuntimeCall::Spawn(surface) => Some(surface.as_str().to_owned()),
                _ => None,
            })
            .collect()
    }

    /// Recorded `attach` surface ids, in call order. A revisit attaches the
    /// existing surface's proxy, replaying scrollback to the fresh sink.
    pub fn attaches(&self) -> Vec<String> {
        self.0
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                RuntimeCall::Attach(surface) => Some(surface.as_str().to_owned()),
                _ => None,
            })
            .collect()
    }
}

/// Like [`test_ctx`] but also returns a [`TestRuntimeProbe`] over the in-memory
/// runtime wired into the `Ctx`, so a host-side test can assert the off-bus
/// runtime writes a command shim performs without a live daemon.
pub async fn test_ctx_with_probe() -> shared::Result<(Ctx, TestRuntimeProbe)> {
    let pool = migrate::open_memory().await?;
    let kv = SqliteKv::new(pool.clone());
    let runtime = Arc::new(FakeRuntime::new());
    let cx = Ctx::new(
        pool,
        kv,
        PathBuf::from("/tmp/tillerd-test-ctx"),
        Runtime::Fake(runtime.clone()),
    );
    Ok((cx, TestRuntimeProbe(runtime)))
}

/// Seed a session row over a test `Ctx` so a host-side test can spawn surfaces
/// (which carry a `session_id` foreign key) without reaching into `infra`. Mirrors
/// the app-layer surface test seed; the default-project sentinel id satisfies the
/// `session.project_id` reference the migrations install.
pub async fn seed_session(cx: &Ctx, id: &str) -> shared::Result<()> {
    sqlx::query("INSERT INTO session (id, project_id, title) VALUES (?, ?, ?)")
        .bind(id)
        .bind("00000000-0000-0000-0000-000000000000")
        .bind("test")
        .execute(cx.db())
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    //
    // These tests exercise the Bus<Ctx> contract produced by build_bus without
    // going through the tracing init (which is process-global). The composition
    // in build_bus is trivially thin; each piece is tested independently.

    async fn memory_ctx() -> Ctx {
        let pool = migrate::open_memory().await.unwrap();
        let kv = SqliteKv::new(pool.clone());
        Ctx::new(
            pool,
            kv,
            PathBuf::from("/tmp/tillerd-boot-test"),
            Runtime::Fake(Arc::new(FakeRuntime::new())),
        )
    }

    struct NoOp;
    impl crate::shared::message::Command<Ctx> for NoOp {
        async fn handle(&self, cx: &Ctx) -> crate::shared::Result<()> {
            let _: i64 = sqlx::query_scalar("SELECT 1").fetch_one(cx.db()).await?;
            Ok(())
        }
    }

    struct CountWorkspaces;
    impl crate::shared::message::Query<Ctx> for CountWorkspaces {
        type Out = i64;
        async fn handle(&self, cx: &Ctx) -> crate::shared::Result<i64> {
            let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspace")
                .fetch_one(cx.db())
                .await?;
            Ok(n)
        }
    }

    struct NoopNotificationSink;
    impl crate::app::notification::NotificationSink for NoopNotificationSink {
        fn emit(&self, _notification: &crate::app::notification::RecordNotification) {}
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
            notification_sink: Arc::new(NoopNotificationSink),
        };

        let bus = build_bus(&cfg).await.unwrap();
        let count = bus.query(CountWorkspaces).await.unwrap();
        assert_eq!(count, 1, "Default workspace seeded after build_bus");
    }

    #[tokio::test]
    async fn an_orchestrator_status_change_at_boot_is_recorded_once_across_a_thread() {
        use std::sync::Mutex;

        use crate::app::notification::{ListNotifications, OrchestratorStatus, RecordNotification};

        struct CountingSink(Arc<Mutex<u32>>);
        impl crate::app::notification::NotificationSink for CountingSink {
            fn emit(&self, _n: &RecordNotification) {
                *self.0.lock().unwrap() += 1;
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let announced = Arc::new(Mutex::new(0u32));
        let cfg = Config {
            db_path: dir.path().join("test.db"),
            socket: dir.path().join("daemon.sock"),
            fs_root: dir.path().join("config"),
            log_dir: dir.path().to_owned(),
            notification_sink: Arc::new(CountingSink(announced.clone())),
        };
        let bus = build_bus(&cfg).await.unwrap();

        // Dispatch from a separate thread that owns its own runtime, mirroring the
        // boot thread. The status change records once via the layer.
        let boot_ctx = bus.cx().clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                Bus::new(boot_ctx)
                    .execute_notable(OrchestratorStatus {
                        ready: true,
                        reason: None,
                        ts: 42,
                    })
                    .await
                    .unwrap();
            });
        })
        .join()
        .unwrap();

        let listing = bus
            .query(ListNotifications {
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].category, "orchestrator-status");
        assert_eq!(*announced.lock().unwrap(), 1);
    }
}
