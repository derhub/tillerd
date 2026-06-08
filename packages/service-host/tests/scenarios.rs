//! Dedicated scenario coverage for the service-host spec (task 8.1).
//!
//! Each test maps 1-to-1 to a named spec scenario; the test name is the scenario.
//!
//! Scenarios that require a live OS signal or a separate process carry
//! `#[ignore]` per the gated-deployment-slice pattern so they appear in
//! `cargo test` output but are skipped in CI unless `--ignored` is passed.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use service_host::host::{run, ServeContext, Service, ServiceConfig};
use service_host::manifest::Manifest;
use service_host::paths::Paths;
use service_host::probe::probe_once;

// ── helpers ──────────────────────────────────────────────────────────────────

fn temp_base(tag: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("/tmp/sh-sc-{tag}-{}-{nanos}", std::process::id())
}

// ── Tool starts via the host entry point ─────────────────────────────────────

/// The host resolves resource paths, writes the manifest, and invokes the
/// tool's serve behavior — in that order.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    PathsResolved,
    ManifestPresent,
    ServeCalled,
}

struct StepRecorder {
    config: ServiceConfig,
    steps: Arc<Mutex<Vec<Step>>>,
}

impl Service for StepRecorder {
    fn config(&self) -> ServiceConfig {
        self.config.clone()
    }

    async fn serve(&mut self, ctx: ServeContext) -> std::io::Result<()> {
        // By the time serve is called:
        // 1. Paths were resolved (base dir exists).
        // 2. Manifest was written (present at the expected path).
        if ctx.paths.base_dir().exists() {
            self.steps.lock().unwrap().push(Step::PathsResolved);
        }
        if Manifest::read(&ctx.paths.manifest_path()).is_some() {
            self.steps.lock().unwrap().push(Step::ManifestPresent);
        }
        self.steps.lock().unwrap().push(Step::ServeCalled);
        Ok(())
    }
}

#[tokio::test]
async fn tool_starts_via_the_host_entry_point() {
    let base = temp_base("entry");
    let steps = Arc::new(Mutex::new(Vec::new()));
    let svc = StepRecorder {
        config: ServiceConfig::new("entry-tool", "1.0.0").with_base_override(Some(base.clone())),
        steps: steps.clone(),
    };

    run(svc).await.unwrap();

    let recorded = steps.lock().unwrap().clone();
    assert_eq!(
        recorded,
        vec![
            Step::PathsResolved,
            Step::ManifestPresent,
            Step::ServeCalled
        ],
        "host resolves paths, writes manifest, then calls serve — in that order"
    );
    let _ = std::fs::remove_dir_all(&base);
}

// ── Paths derived from the base directory ────────────────────────────────────

#[tokio::test]
async fn paths_derived_from_the_base_directory() {
    let base = temp_base("paths");
    let svc = StepRecorder {
        config: ServiceConfig::new("my-tool", "2.0.0").with_base_override(Some(base.clone())),
        steps: Arc::new(Mutex::new(Vec::new())),
    };

    run(svc).await.unwrap();

    let paths = Paths::resolve("my-tool", Some(&base));
    assert!(
        paths.manifest_path().starts_with(&base),
        "manifest path rooted at the base directory"
    );
    assert!(
        paths.socket_path().starts_with(&base),
        "socket path rooted at the base directory"
    );
    let _ = std::fs::remove_dir_all(&base);
}

// ── Base-directory override respected ────────────────────────────────────────

#[tokio::test]
async fn base_directory_override_respected() {
    let base = temp_base("override");
    let svc = StepRecorder {
        config: ServiceConfig::new("override-tool", "1.0.0").with_base_override(Some(base.clone())),
        steps: Arc::new(Mutex::new(Vec::new())),
    };

    run(svc).await.unwrap();

    let paths = Paths::resolve("override-tool", Some(&base));
    // All derived paths must start at the overridden directory.
    assert!(paths.manifest_path().starts_with(&base));
    assert!(paths.socket_path().starts_with(&base));
    assert!(paths.health_socket_path().starts_with(&base));
    let _ = std::fs::remove_dir_all(&base);
}

// ── Manifest written atomically on start ─────────────────────────────────────

struct ManifestWitnessService {
    config: ServiceConfig,
    manifest_pid: Arc<Mutex<Option<u32>>>,
    manifest_version: Arc<Mutex<Option<String>>>,
}

impl Service for ManifestWitnessService {
    fn config(&self) -> ServiceConfig {
        self.config.clone()
    }

    async fn serve(&mut self, ctx: ServeContext) -> std::io::Result<()> {
        if let Some(data) = Manifest::read(&ctx.paths.manifest_path()) {
            *self.manifest_pid.lock().unwrap() = Some(data.pid);
            *self.manifest_version.lock().unwrap() = Some(data.version);
        }
        Ok(())
    }
}

#[tokio::test]
async fn manifest_written_atomically_on_start() {
    let base = temp_base("atomic");
    let manifest_pid = Arc::new(Mutex::new(None::<u32>));
    let manifest_version = Arc::new(Mutex::new(None::<String>));
    let svc = ManifestWitnessService {
        config: ServiceConfig::new("atomic-tool", "3.0.0").with_base_override(Some(base.clone())),
        manifest_pid: manifest_pid.clone(),
        manifest_version: manifest_version.clone(),
    };

    run(svc).await.unwrap();

    let pid = manifest_pid.lock().unwrap().expect("pid read during serve");
    let version = manifest_version
        .lock()
        .unwrap()
        .clone()
        .expect("version read during serve");
    assert_eq!(pid, std::process::id(), "manifest carries the hosting pid");
    assert_eq!(version, "3.0.0", "manifest carries the configured version");
    let _ = std::fs::remove_dir_all(&base);
}

// ── Manifest removed on clean stop ───────────────────────────────────────────

struct NopService {
    config: ServiceConfig,
}

impl Service for NopService {
    fn config(&self) -> ServiceConfig {
        self.config.clone()
    }

    async fn serve(&mut self, _ctx: ServeContext) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn manifest_removed_on_clean_stop() {
    let base = temp_base("clean");
    let svc = NopService {
        config: ServiceConfig::new("clean-tool", "1.0.0").with_base_override(Some(base.clone())),
    };

    run(svc).await.unwrap();

    let paths = Paths::resolve("clean-tool", Some(&base));
    assert!(
        Manifest::read(&paths.manifest_path()).is_none(),
        "manifest must be absent after a clean stop"
    );
    let _ = std::fs::remove_dir_all(&base);
}

// ── Graceful shutdown on signal ───────────────────────────────────────────────

/// Signal-driven shutdown: SIGTERM races `serve`; the host shuts down, cleans
/// up the manifest, and exits. Requires a live process (self-signals), so this
/// test is skipped in headless CI unless run explicitly with `--ignored`.
///
/// Note: sending SIGTERM to the test process itself will fire the handler
/// installed for this test run; this is safe in single-test execution.
#[tokio::test]
#[ignore = "sends SIGTERM to the test process; run in isolation with --ignored"]
async fn graceful_shutdown_on_signal() {
    let base = temp_base("signal");
    let serve_started = Arc::new(Mutex::new(false));
    let serve_started_clone = serve_started.clone();

    struct BlockingService {
        config: ServiceConfig,
        serve_started: Arc<Mutex<bool>>,
    }

    impl Service for BlockingService {
        fn config(&self) -> ServiceConfig {
            self.config.clone()
        }

        async fn serve(&mut self, _ctx: ServeContext) -> std::io::Result<()> {
            *self.serve_started.lock().unwrap() = true;
            // Hang forever; the stop signal races this.
            std::future::pending::<()>().await;
            Ok(())
        }
    }

    let svc = BlockingService {
        config: ServiceConfig::new("signal-tool", "1.0.0").with_base_override(Some(base.clone())),
        serve_started: serve_started_clone,
    };

    let handle = tokio::spawn(run(svc));

    // Wait until serve has started, then send SIGTERM.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !*serve_started.lock().unwrap() && std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        *serve_started.lock().unwrap(),
        "serve must start before signal"
    );

    let _ = std::process::Command::new("kill")
        .args(["-TERM", &std::process::id().to_string()])
        .status();

    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("host completes within timeout after signal")
        .expect("task joined")
        .expect("run returns Ok");

    let paths = Paths::resolve("signal-tool", Some(&base));
    assert!(
        Manifest::read(&paths.manifest_path()).is_none(),
        "manifest cleaned up after signal-driven shutdown"
    );
    let _ = std::fs::remove_dir_all(&base);
}

// ── Launcher probes before connecting ────────────────────────────────────────

struct ProbeWitnessService {
    config: ServiceConfig,
}

impl Service for ProbeWitnessService {
    fn config(&self) -> ServiceConfig {
        self.config.clone()
    }

    async fn serve(&mut self, ctx: ServeContext) -> std::io::Result<()> {
        // Probe the health socket during serve — no token supplied.
        let (status, body) = probe_once(
            &ctx.paths.health_socket_path(),
            "GET /health HTTP/1.1\r\n\r\n",
        )
        .await?;
        assert_eq!(status, "200 OK", "probe answers without a credential");
        assert!(body.contains("\"reachable\":true"), "reports reachable");
        assert!(body.contains("\"version\":\"5.0.0\""), "reports version");
        Ok(())
    }
}

#[tokio::test]
async fn launcher_probes_before_connecting_reports_reachability_and_version() {
    let base = temp_base("probe");
    let svc = ProbeWitnessService {
        config: ServiceConfig::new("probe-tool", "5.0.0").with_base_override(Some(base.clone())),
    };

    run(svc).await.unwrap();
    let _ = std::fs::remove_dir_all(&base);
}
