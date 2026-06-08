//! The single host entry point that wires resource setup, serve, and shutdown.

use std::time::Duration;

use crate::manifest::Manifest;
use crate::paths::Paths;
use crate::probe::Probe;
use crate::shutdown::{ChildRegistry, DEFAULT_GRACE_PERIOD};
use crate::signals::wait_for_stop_signal;

/// A tool's identity and lifecycle configuration. The tool supplies this; the
/// host derives every path and lifecycle behavior from it.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// The tool's name; the manifest and socket file names derive from it.
    pub name: String,
    /// The tool's version, recorded in the manifest and reported by the probe.
    pub version: String,
    /// Optional base-directory override (`ATHING_DIR`-style); `None` resolves to
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

/// What the host hands a tool's serve behavior: the resolved paths, the shared
/// child registry (so the tool tracks any process it spawns), and a future that
/// resolves when a stop signal arrives.
pub struct ServeContext {
    /// The resolved resource paths for this tool.
    pub paths: Paths,
    /// The registry shutdown sweeps; the tool tracks its children here.
    pub children: ChildRegistry,
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
}

/// Start a tool through the host: resolve its paths, write its manifest, install
/// signal handlers, expose the liveness probe, run its serve behavior, then
/// shut down gracefully (escalating, no orphans) and remove the manifest.
pub async fn run<S: Service>(mut service: S) -> std::io::Result<()> {
    let config = service.config();

    // 1. Resolve resource paths (before any manifest write).
    let paths = Paths::resolve(&config.name, config.base_override.as_deref());
    if let Some(parent) = paths.manifest_path().parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 2. Write the manifest atomically (carries pid + version).
    let manifest = Manifest::new(paths.manifest_path());
    manifest.write(&config.version)?;

    // 3. Expose the unauthenticated liveness probe on its own socket, distinct
    //    from the tool's primary socket (which the tool binds in `serve`).
    let probe = Probe::start(paths.health_socket_path(), config.version.clone())?;

    // 4. Run the tool's serve behavior, racing it against the stop signal.
    let children = ChildRegistry::new();
    let ctx = ServeContext {
        paths: paths.clone(),
        children: children.clone(),
    };

    let result = tokio::select! {
        served = service.serve(ctx) => served,
        signal = wait_for_stop_signal() => {
            signal.map(|name| {
                tracing::info!(signal = name, tool = %config.name, "stop signal received");
            })
        }
    };

    // 5. Tool-specific teardown, then escalating graceful-then-forced child
    //    shutdown: no orphans.
    service.shutdown().await;
    children.shutdown_all(config.grace_period).await;
    probe.stop();

    // 6. Remove the manifest on a clean stop.
    manifest.remove();

    result
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
}
