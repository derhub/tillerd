//! The log subscribe command. Host-constructed, never an invoke DTO: it carries
//! an `Arc<dyn LogSink>` (not `Deserialize`) the transport shim builds, so the
//! wire-DTO conventions (`Deserialize`, primitive-only fields) do not apply. It
//! and its handler therefore live in an inner module, outside the file-scoped
//! position the message-dto rules match.

mod command {
    use std::sync::Arc;

    use crate::context::Ctx;
    use crate::events::log::LogSink;
    use crate::shared::message::Command;
    use crate::shared::Result;

    /// Register a client sink under a service key (the log file-name prefix).
    /// Dispatched through the bus once; afterwards the log follower delivers every
    /// appended line for that service straight to the sink, with no per-line
    /// dispatch.
    pub struct SubscribeLogs {
        pub service: String,
        pub sink: Arc<dyn LogSink>,
    }

    impl Command<Ctx> for SubscribeLogs {
        async fn handle(&self, cx: &Ctx) -> Result<()> {
            cx.log_sinks().register(&self.service, self.sink.clone());
            Ok(())
        }
    }

    /// Tear down a log subscription: drop every sink registered under the service
    /// key. Dispatched on close; afterwards the follower finds no sink for the
    /// service and the stream goes quiet.
    pub struct UnsubscribeLogs {
        pub service: String,
    }

    impl Command<Ctx> for UnsubscribeLogs {
        async fn handle(&self, cx: &Ctx) -> Result<()> {
            cx.log_sinks().remove_key(&self.service);
            Ok(())
        }
    }
}

pub use command::{SubscribeLogs, UnsubscribeLogs};

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::app::logs::follow::LogFollower;
    use crate::boot::test_ctx;
    use crate::events::log::{LogLine, LogSink};
    use crate::shared::bus::Bus;

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

    #[tokio::test]
    async fn a_subscribed_sink_receives_appended_lines_for_its_service() {
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
        append(&file, "live-line\n");
        follower.read_appended(&file).await;

        assert_eq!(seen.lock().unwrap().as_slice(), ["live-line"]);
    }

    #[tokio::test]
    async fn an_appended_line_for_another_service_does_not_reach_the_sink() {
        let dir = tempfile::tempdir().unwrap();
        let gate = dir.path().join("tillerd-gate.2026-06-25.log");
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
        append(&gate, "gate-line\n");
        follower.read_appended(&gate).await;

        assert!(seen.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn after_unsubscribing_no_further_lines_reach_the_sink() {
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
        bus.execute(UnsubscribeLogs {
            service: "tillerd-daemon".to_owned(),
        })
        .await
        .unwrap();

        let mut follower = LogFollower::new(dir.path().to_owned(), Arc::clone(ctx.log_sinks()));
        append(&file, "after-unsub\n");
        follower.read_appended(&file).await;

        assert!(seen.lock().unwrap().is_empty());
    }
}
