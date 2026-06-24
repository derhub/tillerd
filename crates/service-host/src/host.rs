//! Host entry point: resource setup, serve, ready/drain lifecycle, shutdown.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;

use crate::manifest::{Manifest, ManifestData, ServiceStatus};
use crate::paths::Paths;
use crate::shutdown::{ChildRegistry, DEFAULT_GRACE_PERIOD};
use crate::signals::{wait_for_drain_signal, wait_for_stop_signal};

/// A tool's identity and lifecycle configuration. The tool supplies this; the
/// host derives every path and lifecycle behavior from it.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// The tool's name; the manifest and socket file names derive from it.
    pub name: String,
    /// The tool's version, recorded in the manifest.
    pub version: String,
    /// Optional base-directory override (`TILLERD_DIR`-style); `None` resolves to
    /// the default base.
    pub base_override: Option<String>,
    /// Grace period before a child that ignores `SIGTERM` is forced.
    pub grace_period: Duration,
}

impl ServiceConfig {
    /// Build a config for a tool with the default grace period.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            base_override: None,
            grace_period: DEFAULT_GRACE_PERIOD,
        }
    }

    /// Set the base-directory override.
    pub fn with_base_override(mut self, base_override: Option<String>) -> Self {
        self.base_override = base_override;
        self
    }
}

/// Readiness handle. The service calls [`Ready::signal`] from its serve behavior once
/// it is listening; the host then flips the manifest `starting -> ready` and logs the transition.
/// Signalling before the host observes it is fine -- the notification is not lost.
#[derive(Clone)]
pub struct Ready {
    notify: Arc<Notify>,
}

impl Ready {
    fn new() -> Self {
        Self {
            notify: Arc::new(Notify::new()),
        }
    }

    /// Announce that serve behavior is accepting work. Idempotent; extra calls are no-ops.
    pub fn signal(&self) {
        self.notify.notify_one();
    }

    async fn wait(&self) {
        self.notify.notified().await;
    }
}

/// Drain handle. The host fires it on the drain signal (SIGUSR2); the service observes
/// it inside serve -- [`Drain::is_draining`] for a fast check, [`Drain::draining`] to await the
/// transition -- then refuses new work, lets active work finish, and returns from serve when idle.
#[derive(Clone, Default)]
pub struct Drain {
    inner: Arc<DrainInner>,
}

#[derive(Default)]
struct DrainInner {
    flag: AtomicBool,
    notify: Notify,
}

impl Drain {
    /// Whether the host has signalled drain.
    pub fn is_draining(&self) -> bool {
        self.inner.flag.load(Ordering::Acquire)
    }

    /// Resolve once drain is signalled (immediately if it already was). Safe to race in a `select!`
    /// alongside the service's accept loop.
    pub async fn draining(&self) {
        if self.is_draining() {
            return;
        }
        // Register for the wakeup before re-checking, so a fire between the check and the await is
        // not lost.
        let notified = self.inner.notify.notified();
        if self.is_draining() {
            return;
        }
        notified.await;
    }

    fn fire(&self) {
        self.inner.flag.store(true, Ordering::Release);
        self.inner.notify.notify_waiters();
    }
}

/// What the host hands a tool's serve behavior: the resolved paths, the shared child registry, and
/// the readiness/drain lifecycle handles.
pub struct ServeContext {
    /// The resolved resource paths for this tool.
    pub paths: Paths,
    /// The registry shutdown sweeps; the tool tracks its children here.
    pub children: ChildRegistry,
    /// Readiness handle: the service calls `ready.signal()` once it is listening.
    pub ready: Ready,
    /// Drain handle: the service observes it to refuse new work and finish active work.
    pub drain: Drain,
}

/// Liveness status from a service's in-process health check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// The service is running and accepting work.
    Serving,
    /// The service has begun graceful shutdown.
    Draining,
}

/// In-process health report: liveness status and version.
///
/// Never serialized over a wire -- health is the service's own concern.
/// The host surfaces it via logging; there is no health socket or route.
#[derive(Debug, Clone)]
pub struct HealthReport {
    /// The service version from its configuration.
    pub version: String,
    /// The service's current liveness status.
    pub status: HealthStatus,
}

/// A long-lived tool. It supplies only its identity and its serve behavior;
/// the host performs path resolution, manifest lifecycle, signal handling, and
/// shutdown on its behalf.
pub trait Service: Send {
    /// The tool's lifecycle configuration.
    fn config(&self) -> ServiceConfig;

    /// Run the tool's serve behavior until told to stop. The host runs this
    /// after resource setup and races it against the stop signal; the tool need
    /// not watch for signals itself.
    fn serve(
        &mut self,
        ctx: ServeContext,
    ) -> impl std::future::Future<Output = std::io::Result<()>> + Send;

    /// Tear down tool-specific resources on stop, before the child sweep. Runs
    /// once `serve` has returned or been cancelled by the stop signal. Default
    /// no-op; tools that own non-child resources (e.g. live sessions) override it.
    fn shutdown(&mut self) -> impl std::future::Future<Output = ()> + Send {
        async {}
    }

    /// In-process health self-check. The host calls this to log the service's
    /// status at startup and during graceful drain.
    ///
    /// The default reports serving at the configured version; override to report
    /// a different status (for example, `Draining` once shutdown begins).
    fn health(&self) -> HealthReport {
        HealthReport {
            version: self.config().version,
            status: HealthStatus::Serving,
        }
    }
}

/// Start a tool through the host: resolve its paths, write its manifest, install
/// signal handlers, run its serve behavior, then shut down gracefully
/// (escalating, no orphans) and remove the manifest.
pub async fn run<S: Service>(mut service: S) -> std::io::Result<()> {
    let config = service.config();

    // 1. Resolve resource paths (before any manifest write).
    let paths = Paths::resolve(&config.name, config.base_override.as_deref());
    if let Some(parent) = paths.manifest_path().parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 2. Write the manifest atomically at `starting`, carrying the socket clients will reach this
    //    service on once it is ready (discovery is manifest-only).
    let manifest = Manifest::new(paths.manifest_path());
    let socket_path = paths.socket_path().to_string_lossy().into_owned();
    let mut manifest_data = ManifestData {
        pid: std::process::id(),
        version: config.version.clone(),
        status: ServiceStatus::Starting,
        socket_path: Some(socket_path),
    };
    manifest.write_data(&manifest_data)?;

    // 3. Run the tool's serve behavior, racing it against ready/drain/stop.
    let children = ChildRegistry::new();
    let ready = Ready::new();
    let drain = Drain::default();
    let ctx = ServeContext {
        paths: paths.clone(),
        children: children.clone(),
        ready: ready.clone(),
        drain: drain.clone(),
    };

    let health = service.health();
    tracing::info!(
        service = %config.name,
        version = %health.version,
        status = ?health.status,
        "service started"
    );

    // ready/drain fire during serve and update lifecycle state without ending it; only a stop
    // signal cancels serve, and only serve returning (idle, including after drain) ends it cleanly.
    // serve is boxed so it can be dropped right after the loop, releasing its `&mut service` borrow
    // before the stopping health-check and teardown re-borrow the service.
    let mut serve = Box::pin(service.serve(ctx));
    let ready_wait = ready.wait();
    let stop = wait_for_stop_signal();
    let drain_wait = wait_for_drain_signal();
    tokio::pin!(ready_wait, stop, drain_wait);

    let mut ready_seen = false;
    let mut drain_seen = false;
    let result = loop {
        tokio::select! {
            // Serve returned on its own (natural exit, or idle after drain): a clean stop.
            served = &mut serve => break served,

            // Readiness: flip starting -> ready and record it.
            _ = &mut ready_wait, if !ready_seen => {
                ready_seen = true;
                manifest_data.status = ServiceStatus::Ready;
                let _ = manifest.write_data(&manifest_data);
                tracing::info!(service = %config.name, status = ?ServiceStatus::Ready, "service ready");
            }

            // Drain (SIGUSR2): refuse new work via the handle, record draining, keep awaiting serve.
            _ = &mut drain_wait, if !drain_seen => {
                drain_seen = true;
                drain.fire();
                manifest_data.status = ServiceStatus::Draining;
                let _ = manifest.write_data(&manifest_data);
                tracing::info!(service = %config.name, status = ?ServiceStatus::Draining, "service draining");
            }

            // Stop (SIGTERM/SIGINT): cancel serve now and tear down.
            signal = &mut stop => break signal.map(|name| {
                tracing::info!(signal = name, service = %config.name, "stop signal received");
            }),
        }
    };

    // Drop the serve future (completed, or cancelled by a stop signal) to release its `&mut service`
    // borrow before the teardown re-borrows the service.
    drop(serve);

    // 4. Tool-specific teardown, then escalating graceful-then-forced child shutdown: no orphans.
    //    serve has returned (or been cancelled), so `&mut service` is free to health-check again.
    let health = service.health();
    tracing::info!(
        service = %config.name,
        version = %health.version,
        status = ?health.status,
        "service stopping"
    );
    service.shutdown().await;
    children.shutdown_all(config.grace_period).await;

    // 5. Remove the manifest on a clean stop.
    manifest.remove();

    result
}

/// Build the standard multi-thread runtime, run the service under the host,
/// and exit with a uniform error message on failure. Every service binary
/// collapses to one call to this function.
pub fn run_blocking<S: Service>(service: S) {
    let name = service.config().name;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| {
            eprintln!("{name}: failed to build runtime: {e}");
            std::process::exit(1);
        });
    if let Err(e) = rt.block_on(run(service)) {
        eprintln!("{name}: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Step {
        ManifestWrittenWith(String),
        Serve,
    }

    struct RecordingService {
        config: ServiceConfig,
        steps: Arc<Mutex<Vec<Step>>>,
    }

    impl Service for RecordingService {
        fn config(&self) -> ServiceConfig {
            self.config.clone()
        }

        async fn serve(&mut self, ctx: ServeContext) -> std::io::Result<()> {
            // Observe whether resource setup ran before serve: the manifest
            // exists and the base directory is resolved by now.
            let manifest = Manifest::read(&ctx.paths.manifest_path())
                .expect("manifest must exist before serve runs");
            self.steps
                .lock()
                .unwrap()
                .push(Step::ManifestWrittenWith(manifest.version));
            self.steps.lock().unwrap().push(Step::Serve);
            Ok(())
        }
    }

    struct HealthOverrideService {
        config: ServiceConfig,
        status: HealthStatus,
    }

    impl Service for HealthOverrideService {
        fn config(&self) -> ServiceConfig {
            self.config.clone()
        }

        async fn serve(&mut self, _ctx: ServeContext) -> std::io::Result<()> {
            Ok(())
        }

        fn health(&self) -> HealthReport {
            HealthReport {
                version: self.config.version.clone(),
                status: self.status.clone(),
            }
        }
    }

    struct HealthTrackingService {
        config: ServiceConfig,
        health_calls: Arc<Mutex<Vec<HealthStatus>>>,
    }

    impl Service for HealthTrackingService {
        fn config(&self) -> ServiceConfig {
            self.config.clone()
        }

        async fn serve(&mut self, _ctx: ServeContext) -> std::io::Result<()> {
            Ok(())
        }

        fn health(&self) -> HealthReport {
            let status = HealthStatus::Serving;
            self.health_calls.lock().unwrap().push(status.clone());
            HealthReport {
                version: self.config.version.clone(),
                status,
            }
        }
    }

    // The host derives a socket path from the base; Unix socket paths are
    // capped (~104 bytes), so keep the base short and under /tmp.
    fn temp_base(tag: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("/tmp/sh-h-{tag}-{}-{nanos}", std::process::id())
    }

    #[tokio::test]
    async fn host_invokes_serve_behavior_after_resource_setup() {
        let base = temp_base("serve-after-setup");
        let steps = Arc::new(Mutex::new(Vec::new()));
        let service = RecordingService {
            config: ServiceConfig::new("toolx", "1.0.0").with_base_override(Some(base.clone())),
            steps: steps.clone(),
        };

        run(service).await.unwrap();

        let recorded = steps.lock().unwrap().clone();
        assert_eq!(
            recorded,
            vec![Step::ManifestWrittenWith("1.0.0".into()), Step::Serve],
            "serve runs only after the manifest (resource setup) is in place"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn host_resolves_base_directory_before_manifest_write() {
        let base = temp_base("resolve-before-write");
        let service = RecordingService {
            config: ServiceConfig::new("tooly", "2.0.0").with_base_override(Some(base.clone())),
            steps: Arc::new(Mutex::new(Vec::new())),
        };

        run(service).await.unwrap();

        // The manifest landed under the resolved base directory, proving the
        // base was resolved before the write targeted it.
        let manifest_path = Paths::resolve("tooly", Some(&base)).manifest_path();
        assert!(manifest_path.starts_with(&base));
        // On a clean stop the manifest is removed; its parent (the resolved
        // base) was created during resolution.
        assert!(std::path::Path::new(&base).exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn health_default_reports_serving_at_configured_version() {
        let service = RecordingService {
            config: ServiceConfig::new("tool", "3.2.1"),
            steps: Arc::new(Mutex::new(Vec::new())),
        };

        let report = service.health();

        assert_eq!(report.version, "3.2.1");
        assert_eq!(report.status, HealthStatus::Serving);
    }

    #[test]
    fn health_override_reports_own_status() {
        let service = HealthOverrideService {
            config: ServiceConfig::new("tool", "1.0.0"),
            status: HealthStatus::Draining,
        };

        let report = service.health();

        assert_eq!(report.status, HealthStatus::Draining);
    }

    #[tokio::test]
    async fn host_calls_health_at_startup_and_stop() {
        let base = temp_base("health-calls");
        let health_calls = Arc::new(Mutex::new(Vec::new()));
        let service = HealthTrackingService {
            config: ServiceConfig::new("toolh", "1.0.0").with_base_override(Some(base.clone())),
            health_calls: health_calls.clone(),
        };

        run(service).await.unwrap();

        assert_eq!(
            health_calls.lock().unwrap().len(),
            2,
            "host calls health() at startup and again when serve returns (stopping)"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn manifest_starts_at_starting_then_flips_to_ready_on_signal() {
        let base = temp_base("ready-flip");
        let manifest_path = Paths::resolve("toolr", Some(&base)).manifest_path();

        // A service that signals ready, observes the manifest flipped to `ready`, then returns.
        struct ReadySignalService {
            config: ServiceConfig,
            observed: Arc<Mutex<Option<ServiceStatus>>>,
        }
        impl Service for ReadySignalService {
            fn config(&self) -> ServiceConfig {
                self.config.clone()
            }
            async fn serve(&mut self, ctx: ServeContext) -> std::io::Result<()> {
                ctx.ready.signal();
                // Give the host a moment to process the ready signal and rewrite the manifest.
                for _ in 0..50 {
                    if Manifest::read(&ctx.paths.manifest_path()).map(|m| m.status)
                        == Some(ServiceStatus::Ready)
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                *self.observed.lock().unwrap() =
                    Manifest::read(&ctx.paths.manifest_path()).map(|m| m.status);
                Ok(())
            }
        }

        let observed = Arc::new(Mutex::new(None));
        run(ReadySignalService {
            config: ServiceConfig::new("toolr", "1.0.0").with_base_override(Some(base.clone())),
            observed: observed.clone(),
        })
        .await
        .unwrap();

        assert_eq!(
            *observed.lock().unwrap(),
            Some(ServiceStatus::Ready),
            "host flips the manifest to ready once the service signals"
        );
        // The manifest carries the socket path while live; on clean stop it is removed.
        assert!(Manifest::read(&manifest_path).is_none());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn drain_signal_flips_handle_and_lets_serve_finish() {
        // Drain fires via the handle (not a real SIGUSR2): the service's serve loop observes it and
        // returns, which the host treats as a clean idle exit.
        let drain = Drain::default();
        assert!(!drain.is_draining());
        drain.fire();
        assert!(drain.is_draining());
        // `draining()` resolves immediately once fired.
        drain.draining().await;
    }
}
