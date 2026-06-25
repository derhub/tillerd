
use std::sync::Arc;

use orchestrator::app::logs::{
    ListLogFiles, LogFileView, LogLine, LogSink, LogTailView, SubscribeLogs, TailLog,
    UnsubscribeLogs,
};
use tauri::{ipc::Channel, State};

use crate::transport::Bus;
use crate::transport::macros::{transport_command, transport_query};


pub struct LogChannelSink {
    channel: Channel<String>,
}

impl LogChannelSink {
    pub fn for_channel(channel: Channel<String>) -> Self {
        Self { channel }
    }
}

impl LogSink for LogChannelSink {
    fn emit(&self, _service: &str, line: &LogLine<'_>) {
        let _ = self.channel.send(line.0.to_owned());
    }
}


#[tauri::command]
#[specta::specta]
pub async fn log_subscribe(
    bus: State<'_, Bus>,
    channel: Channel<String>,
    service: String,
) -> Result<(), String> {
    let sink: Arc<dyn LogSink> = Arc::new(LogChannelSink::for_channel(channel));
    bus.execute(SubscribeLogs { service, sink })
        .await
        .map_err(|e| e.to_string())
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

transport_command!(log_unsubscribe(service: String) => UnsubscribeLogs { service });

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;


    fn recording_line_channel() -> (tauri::ipc::Channel<String>, Arc<Mutex<Vec<String>>>) {
        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = received.clone();
        let channel = tauri::ipc::Channel::new(move |body| {
            if let tauri::ipc::InvokeResponseBody::Json(json) = body {
                let line: String = serde_json::from_str(&json).unwrap_or_default();
                sink.lock().unwrap().push(line);
            }
            Ok(())
        });
        (channel, received)
    }


    #[test]
    fn log_sink_sends_each_line_to_the_channel() {
        let (channel, received) = recording_line_channel();
        let sink = LogChannelSink::for_channel(channel);

        sink.emit("tillerd-daemon", &LogLine("a line"));

        assert_eq!(received.lock().unwrap().as_slice(), ["a line".to_owned()]);
    }
}
