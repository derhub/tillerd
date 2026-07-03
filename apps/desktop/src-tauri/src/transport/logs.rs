use orchestrator::app::logs::{ListLogFiles, LogFileView, LogTailView, TailLog};

use crate::transport::macros::{domain_channel, transport_query};

domain_channel! {
    pub open log_channel(orchestrator::app::logs::OpenLogChannel),
    pub close log_channel_close(orchestrator::app::logs::CloseLogChannel)
}

domain_channel! {
    pub open logs_changed_channel(orchestrator::app::logs::OpenLogsChangedChannel),
    pub close logs_changed_channel_close(orchestrator::app::logs::CloseLogsChangedChannel)
}

transport_query!(
    log_list() -> Vec<LogFileView>
        => ListLogFiles,
        |files| files
);

transport_query!(
    log_tail(path: String, from: u64, max_bytes: u64, align: bool) -> LogTailView
        => TailLog { path, from, max_bytes, align },
        |view| view
);

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use orchestrator::app::logs::{CloseLogChannel, OpenLogChannel};
    use orchestrator::boot::test_ctx;
    use orchestrator::shared::domain_channel::{
        CloseDomainChannel, DomainChannelEvent, DomainChannelSink, OpenDomainChannel,
    };

    struct Recorder(Arc<Mutex<Vec<String>>>);
    impl DomainChannelSink for Recorder {
        fn emit(&self, event: &DomainChannelEvent<'_>) {
            if let DomainChannelEvent::Bytes(b) = event {
                self.0
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(b).into_owned());
            }
        }
    }

    #[tokio::test]
    async fn open_log_channel_registers_sink_in_registry() {
        let cx = test_ctx().await.unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));

        let open_cmd = OpenLogChannel {
            service: "tillerd-daemon".to_owned(),
        };

        open_cmd
            .handle(&cx, Arc::new(Recorder(seen.clone())))
            .await
            .unwrap();

        cx.domain_channel_sinks()
            .dispatch("logs://tillerd-daemon", |s| {
                s.emit(&DomainChannelEvent::Bytes(b"log-line"))
            });

        assert_eq!(seen.lock().unwrap().as_slice(), &["log-line".to_owned()]);
    }

    #[tokio::test]
    async fn close_log_channel_removes_sink_from_registry() {
        let cx = test_ctx().await.unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));

        let open_cmd = OpenLogChannel {
            service: "tillerd-daemon".to_owned(),
        };

        open_cmd
            .handle(&cx, Arc::new(Recorder(seen.clone())))
            .await
            .unwrap();

        let close_cmd = CloseLogChannel {
            service: "tillerd-daemon".to_owned(),
        };

        close_cmd.handle(&cx).await.unwrap();

        cx.domain_channel_sinks()
            .dispatch("logs://tillerd-daemon", |s| {
                s.emit(&DomainChannelEvent::Bytes(b"log-line"))
            });

        assert!(seen.lock().unwrap().is_empty());
    }
}
