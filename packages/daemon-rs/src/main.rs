#![deny(unsafe_code)]

mod cell;
mod codec;
mod exit_qualifier;
mod hook_ingress;
mod manifest;
mod messages;
mod pty_session;
mod replay;
mod resolve;
mod server;
mod shell_env;
mod signals;
mod snapshot;
mod stopped_sessions;
mod vt;

use manifest::{athing_dir, daemon_sock, hooks_sock, Manifest};
use server::{Daemon, DAEMON_VERSION};
use tokio::sync::mpsc::unbounded_channel;
use tracing::Instrument;

const SERVICE_NAME: &str = "athing-daemon";

// Keeps the non-blocking log writer's worker thread alive for the process lifetime.
static LOG_GUARD: std::sync::OnceLock<tracing_appender::non_blocking::WorkerGuard> =
    std::sync::OnceLock::new();

// Structured JSON logging to ATHING_DIR/logs/daemon.<date>.log, separate from the
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

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    rt.block_on(async_main());
}

async fn async_main() {
    let args: Vec<String> = std::env::args().collect();
    let is_handoff = args.iter().any(|a| a == "--handoff");

    if !is_handoff {
        shell_env::install_login_shell_env();
    }

    let dir = athing_dir();
    let _ = std::fs::create_dir_all(&dir);

    init_tracing(&dir);
    let root = tracing::info_span!(
        "daemon",
        service.name = SERVICE_NAME,
        service.version = DAEMON_VERSION,
        process.pid = std::process::id(),
    );

    let sock = arg_value(&args, "--socket")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| daemon_sock(&dir));
    let sock_for_poll = sock.clone();
    let hooks = hooks_sock(&dir);

    let (events_tx, events_rx) = unbounded_channel();
    let daemon = Daemon::new(&dir, events_tx).with_paths(sock, hooks);

    if is_handoff {
        let _g = root.enter();
        match arg_value(&args, "--snapshot") {
            Some(snap) => match snapshot::read_snapshot(std::path::Path::new(&snap)) {
                Ok(records) => {
                    let n = daemon.adopt_records(&records);
                    tracing::info!(sessions = n, "handoff adopted sessions");
                }
                Err(e) => {
                    tracing::error!(error = %e, "handoff: snapshot read failed; starting empty")
                }
            },
            None => tracing::warn!("handoff: --snapshot missing; starting empty"),
        }
    }

    let manifest = Manifest::new(dir.clone());

    // Graceful shutdown on SIGTERM/SIGINT: cascade-terminate sessions, remove
    // the manifest, exit with no orphaned children.
    {
        let daemon = daemon.clone();
        let manifest_dir = dir.clone();
        tokio::spawn(async move {
            let mut term =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
            let mut int =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
            tokio::select! {
                _ = term.recv() => {}
                _ = int.recv() => {}
            }
            daemon.shutdown();
            Manifest::new(manifest_dir).remove();
            std::process::exit(0);
        });
    }

    // Run the accept loop; write the manifest only AFTER the control socket is
    // bound, so a predecessor handing off to us sees our pid only once we are
    // actually serving (its handoff ack polls the manifest pid).
    let serve_daemon = daemon.clone();
    let serve_handle = tokio::spawn(
        async move {
            if let Err(e) = serve_daemon.serve(events_rx).await {
                tracing::error!(error = %e, "daemon serve error");
                std::process::exit(1);
            }
        }
        .instrument(root.clone()),
    );

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !sock_for_poll.exists() && std::time::Instant::now() < deadline {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    manifest.write(DAEMON_VERSION).ok();

    let _ = serve_handle.await;
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    let prefix = format!("{flag}=");
    args.iter()
        .find_map(|a| a.strip_prefix(&prefix).map(|s| s.to_string()))
}
