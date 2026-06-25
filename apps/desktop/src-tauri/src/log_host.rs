//! Tauri bridge for the live log stream. Output-only: the renderer passes an
//! `ipc::Channel<String>`, `log_subscribe` wraps it in a `LogChannelSink` and
//! registers it under a service key via the bus (`SubscribeLogs`); the orchestrator
//! log follower then delivers every appended line for that service straight to the
//! channel. `log_unsubscribe` drops the registration (`UnsubscribeLogs`). Logs have
//! no client->backend direction, so one subscribe + one teardown is the whole shape.

use std::sync::Arc;

use orchestrator::app::logs::{LogSink, SubscribeLogs, UnsubscribeLogs};
use orchestrator::shared::Bus;
use orchestrator::Ctx;
use tauri::State;

use crate::transport::sink::LogChannelSink;

/// Register the renderer's channel as a live sink for `service`'s appended log
/// lines. Subsequent lines arrive on `channel` until `log_unsubscribe`.
#[tauri::command]
#[specta::specta]
pub async fn log_subscribe(
    bus: State<'_, Bus<Ctx>>,
    channel: tauri::ipc::Channel<String>,
    service: String,
) -> Result<(), String> {
    let sink: Arc<dyn LogSink> = Arc::new(LogChannelSink::for_channel(channel));
    bus.execute(SubscribeLogs { service, sink })
        .await
        .map_err(|e| e.to_string())
}

/// Tear down the live log subscription for `service`: drop every sink registered
/// under the service key so the stream goes quiet.
#[tauri::command]
#[specta::specta]
pub async fn log_unsubscribe(bus: State<'_, Bus<Ctx>>, service: String) -> Result<(), String> {
    bus.execute(UnsubscribeLogs { service })
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use orchestrator::app::logs::{LogFollower, LogLine, LogSink};
    use orchestrator::boot::test_ctx;
    use orchestrator::shared::Bus;

    use super::*;

    /// A sink that records the lines it is emitted for.
    struct Recorder(Arc<Mutex<Vec<String>>>);
    impl LogSink for Recorder {
        fn emit(&self, _service: &str, line: &LogLine<'_>) {
            self.0.lock().unwrap().push(line.0.to_owned());
        }
    }

    fn append(path: &std::path::Path, text: &str) {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        f.write_all(text.as_bytes()).unwrap();
        f.flush().unwrap();
    }

    // A subscribed service receives its appended lines through the follower; after
    // unsubscribe the registration is gone and no further line is delivered.
    #[tokio::test]
    async fn subscribe_then_unsubscribe_starts_and_stops_delivery() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("tillerd-daemon.2026-06-25.log");
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let ctx = test_ctx().await.unwrap();
        let bus = Bus::new(ctx.clone());

        bus.execute(SubscribeLogs {
            service: "tillerd-daemon".to_owned(),
            sink: Arc::new(Recorder(Arc::clone(&seen))),
        })
        .await
        .unwrap();

        let mut follower = LogFollower::new(dir.path().to_owned(), Arc::clone(ctx.log_sinks()));
        append(&file, "first\n");
        follower.read_appended(&file).await;
        assert_eq!(seen.lock().unwrap().as_slice(), ["first"]);

        bus.execute(UnsubscribeLogs {
            service: "tillerd-daemon".to_owned(),
        })
        .await
        .unwrap();

        append(&file, "second\n");
        follower.read_appended(&file).await;
        assert_eq!(seen.lock().unwrap().as_slice(), ["first"]);
    }
}
