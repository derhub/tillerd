#![deny(unsafe_code)]

mod codec;
mod exit_qualifier;
mod manifest;
mod messages;
mod pty_session;
mod replay;
mod resolve;
mod server;
mod shell_env;
mod signals;
mod stopped_sessions;

use server::{Daemon, DAEMON_VERSION};
use service_host::host::{ServeContext, Service, ServiceConfig};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tracing::Instrument;

const SERVICE_NAME: &str = "tillerd-daemon";

// Keeps the non-blocking log writer's worker thread alive for the process lifetime.
static LOG_GUARD: std::sync::OnceLock<tracing_appender::non_blocking::WorkerGuard> =
    std::sync::OnceLock::new();

// Structured JSON logging to TILLERD_DIR/logs/daemon.<date>.log, separate from the
// TypeScript runtime's log file. OTLP export can be layered in later behind this same init.
fn init_tracing(dir: &std::path::Path) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let logs_dir = dir.join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);

    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("daemon")
        .filename_suffix("log")
        .build(&logs_dir)
        .expect("daemon log appender");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let _ = LOG_GUARD.set(guard);

    // LOG_LEVEL mirrors the TS logger; "silent" maps to no output.
    let level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into());
    let directive = if level.eq_ignore_ascii_case("silent") {
        "off".to_string()
    } else {
        level
    };
    let filter = EnvFilter::try_new(&directive).unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true)
                .with_writer(writer),
        )
        .init();
}

// The daemon as a hosted Service: service-host owns path/manifest/signal
// lifecycle; the daemon binds its own control socket in `serve` and tears down
// live PTY sessions in `shutdown` (they are not service-host children).
struct DaemonService {
    daemon: Daemon,
    events_rx: Option<UnboundedReceiver<pty_session::SessionEvent>>,
    root: tracing::Span,
}

impl DaemonService {
    fn from_env() -> Self {
        shell_env::install_login_shell_env();

        let dir = tillerd_paths::runtime_dir();
        let _ = std::fs::create_dir_all(&dir);

        init_tracing(&dir);
        let root = tracing::info_span!(
            "daemon",
            service.name = SERVICE_NAME,
            service.version = DAEMON_VERSION,
            process.pid = std::process::id(),
        );

        let (events_tx, events_rx) = unbounded_channel();
        let daemon = Daemon::new(&dir, events_tx);

        Self {
            daemon,
            events_rx: Some(events_rx),
            root,
        }
    }
}

impl Service for DaemonService {
    fn config(&self) -> ServiceConfig {
        ServiceConfig::new("daemon", DAEMON_VERSION)
            .with_base_override(std::env::var(tillerd_paths::ENV_TILLERD_DIR).ok())
    }

    async fn serve(&mut self, ctx: ServeContext) -> std::io::Result<()> {
        let events_rx = self.events_rx.take().expect("serve runs once");
        self.daemon
            .serve(events_rx, ctx.ready, ctx.drain)
            .instrument(self.root.clone())
            .await
    }

    async fn shutdown(&mut self) {
        self.daemon.shutdown();
    }
}

fn main() {
    service_host::run_blocking(DaemonService::from_env());
}
