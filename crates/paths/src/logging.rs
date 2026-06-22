//! Shared structured-logging init. Every long-lived service writes JSON log
//! records to a rolling, per-service file under the runtime logs directory, so a
//! single viewer can read them all.

use std::path::{Path, PathBuf};

use tracing::Span;
use tracing_appender::non_blocking::WorkerGuard;

/// The logs directory under `dir`.
pub fn logs_dir_in(dir: &Path) -> PathBuf {
    dir.join("logs")
}

/// Initialize process-global structured JSON logging to
/// `<dir>/logs/<service>.<date>.log`, rotated daily.
///
/// Returns the writer guard (hold it for the process lifetime, dropping it
/// flushes the non-blocking writer) and a root span carrying the
/// `service.name` / `service.version` resource; enter or instrument with the
/// span so every record carries the resource. Honors `LOG_LEVEL` as an
/// `EnvFilter` directive (`silent` maps to `off`, default `info`).
///
/// Sets the global default subscriber on the first call; a subsequent call is a
/// no-op for the subscriber (the existing one stays installed) but still returns
/// a fresh writer guard and root span for the caller's service identity. This
/// keeps a process that hosts more than one service (the desktop app embedding
/// the orchestrator) from panicking on the second initialization.
pub fn init_file_tracing(service: &str, version: &str, dir: &Path) -> (WorkerGuard, Span) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let logs_dir = logs_dir_in(dir);
    let _ = std::fs::create_dir_all(&logs_dir);

    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(service)
        .filename_suffix("log")
        .build(&logs_dir)
        .expect("log appender");
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into());
    let directive = if level.eq_ignore_ascii_case("silent") {
        "off".to_string()
    } else {
        level
    };
    let filter = EnvFilter::try_new(&directive).unwrap_or_else(|_| EnvFilter::new("info"));

    // `try_init` rather than `init`: a process that hosts two services (the desktop
    // app embeds the orchestrator) calls this twice. The first call wins; the second
    // is a no-op instead of a panic.
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true)
                .with_writer(writer),
        )
        .try_init();

    let root = tracing::info_span!("service", service.name = service, service.version = version);
    (guard, root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn logs_dir_is_under_the_runtime_dir() {
        assert_eq!(
            logs_dir_in(Path::new("/srv/tillerd")),
            PathBuf::from("/srv/tillerd/logs")
        );
    }

    #[test]
    #[serial]
    fn writes_one_json_record_carrying_the_service_resource() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("LOG_LEVEL", "info");
        let (guard, root) = init_file_tracing("test-svc", "9.9.9", dir.path());
        {
            let _entered = root.enter();
            tracing::info!("hello from test");
        }
        drop(guard); // flush the non-blocking writer

        let logs = logs_dir_in(dir.path());
        let file = std::fs::read_dir(&logs)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("test-svc") && n.ends_with("log"))
            })
            .expect("dated per-service log file exists");

        let contents = std::fs::read_to_string(&file).unwrap();
        let line = contents.lines().next().expect("at least one record");
        let record: serde_json::Value = serde_json::from_str(line).expect("record is valid JSON");
        assert!(line.contains("hello from test"), "body present: {line}");
        assert!(line.contains("test-svc"), "service.name present: {line}");
        assert!(record.is_object(), "record is a JSON object");
    }
}
