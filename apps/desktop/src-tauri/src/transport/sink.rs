//! The tauri implementation of the surface output sink. Each surface subscription
//! owns one `ChannelSink`: bytes are written to the per-surface
//! `tauri::ipc::Channel<Vec<u8>>` the renderer provided; status/exit/error are
//! forwarded as tauri events. The orchestrator registry keys sinks by surface id,
//! so this sink ignores the routing arg and serves its single channel. A future
//! web transport implements the same `SurfaceSink` port with SSE/WebSocket.
//!
//! Keystroke input never flows through this sink -- it carries daemon -> renderer
//! output only (see the off-bus input endpoints), so no payload is ever logged here.

use orchestrator::app::logs::{LogLine, LogSink};
use orchestrator::app::surface::{SurfaceEvent, SurfaceSink};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};

/// The status event name (surface lifecycle: live/idle/failed).
pub const STATUS_EVENT: &str = "surface://status";
/// The exit event name (the PTY's process ended).
pub const EXIT_EVENT: &str = "surface://exit";
/// The error event name (a non-recoverable surface error after open).
pub const ERROR_EVENT: &str = "surface:error";

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "surface://status")]
#[serde(rename_all = "camelCase")]
pub struct SurfaceStatusPayload {
    pub surface_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "surface://exit")]
#[serde(rename_all = "camelCase")]
pub struct SurfaceExitPayload {
    pub surface_id: String,
    pub qualifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "surface:error")]
#[serde(rename_all = "camelCase")]
pub struct SurfaceErrorPayload {
    pub surface_id: String,
    pub reason: String,
}

/// Bridges one surface's runtime output to the renderer over tauri IPC. Bytes go
/// to the renderer-provided `Channel`; lifecycle frames are emitted as events.
/// Generic over the runtime so it works under both `Wry` and the `tauri::test`
/// mock runtime. The registry owns key-scoping, so `emit`'s surface arg is used
/// only to address the lifecycle event payloads, never to route bytes.
pub struct ChannelSink<R: Runtime> {
    channel: tauri::ipc::Channel<Vec<u8>>,
    app: AppHandle<R>,
}

impl<R: Runtime> ChannelSink<R> {
    /// Build a sink over one renderer channel and the app handle it emits events
    /// through.
    pub fn for_channel(channel: tauri::ipc::Channel<Vec<u8>>, app: AppHandle<R>) -> Self {
        Self { channel, app }
    }
}

impl<R: Runtime> SurfaceSink for ChannelSink<R> {
    fn emit(&self, surface: &str, event: &SurfaceEvent<'_>) {
        match event {
            SurfaceEvent::Bytes(bytes) => {
                let _ = self.channel.send(bytes.to_vec());
            }
            SurfaceEvent::Status(status) => {
                let _ = self.app.emit(
                    STATUS_EVENT,
                    serde_json::json!({ "surfaceId": surface, "status": status }),
                );
            }
            SurfaceEvent::Exit(qualifier) => {
                let _ = self.app.emit(
                    EXIT_EVENT,
                    serde_json::json!({ "surfaceId": surface, "qualifier": qualifier }),
                );
            }
            SurfaceEvent::Error(reason) => {
                let _ = self.app.emit(
                    ERROR_EVENT,
                    serde_json::json!({ "surfaceId": surface, "reason": reason }),
                );
            }
        }
    }
}

/// Bridges one service's followed log output to the renderer over tauri IPC.
/// Each appended line is sent as a `String` on the renderer-provided `Channel`.
/// The registry owns key-scoping, so `emit`'s service arg is unused for routing.
pub struct LogChannelSink {
    channel: tauri::ipc::Channel<String>,
}

impl LogChannelSink {
    /// Build a sink over one renderer channel.
    pub fn for_channel(channel: tauri::ipc::Channel<String>) -> Self {
        Self { channel }
    }
}

impl LogSink for LogChannelSink {
    fn emit(&self, _service: &str, line: &LogLine<'_>) {
        let _ = self.channel.send(line.0.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// Captured payloads delivered to a recording channel, shared with the test.
    type Recorded = Arc<Mutex<Vec<Vec<u8>>>>;

    /// A `Channel` that records every payload it receives, so a test can assert
    /// what the sink delivered without a running tauri app.
    fn recording_channel() -> (tauri::ipc::Channel<Vec<u8>>, Recorded) {
        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = received.clone();
        let channel = tauri::ipc::Channel::new(move |body| {
            // `Channel::send(Vec<u8>)` serializes through the blanket `Serialize`
            // path, so the body is a JSON array of byte values.
            if let tauri::ipc::InvokeResponseBody::Json(json) = body {
                let bytes: Vec<u8> = serde_json::from_str(&json).unwrap_or_default();
                sink.lock().unwrap().push(bytes);
            }
            Ok(())
        });
        (channel, received)
    }

    // A `Bytes` event is delivered to the sink's own channel.
    #[test]
    fn emit_sends_bytes_to_the_channel() {
        let (channel, received) = recording_channel();
        let sink = sink_over(channel);

        sink.emit("surf-1", &SurfaceEvent::Bytes(b"hello"));

        assert_eq!(received.lock().unwrap().as_slice(), [b"hello".to_vec()]);
    }

    // Lifecycle frames are emitted as events, not pushed to the byte channel.
    #[test]
    fn emit_does_not_push_lifecycle_frames_to_the_channel() {
        let (channel, received) = recording_channel();
        let sink = sink_over(channel);

        sink.emit("surf-1", &SurfaceEvent::Status("live"));
        sink.emit("surf-1", &SurfaceEvent::Exit("0"));
        sink.emit("surf-1", &SurfaceEvent::Error("boom"));

        assert!(received.lock().unwrap().is_empty());
    }

    /// Build a sink over a recording channel using the `tauri::test` mock app.
    fn sink_over(channel: tauri::ipc::Channel<Vec<u8>>) -> ChannelSink<tauri::test::MockRuntime> {
        let app = tauri::test::mock_app();
        ChannelSink::for_channel(channel, app.handle().clone())
    }

    /// A `Channel<String>` that records every line it receives.
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

    // A followed line is sent to the log sink's own channel as a string.
    #[test]
    fn log_sink_sends_each_line_to_the_channel() {
        let (channel, received) = recording_line_channel();
        let sink = LogChannelSink::for_channel(channel);

        sink.emit("tillerd-daemon", &LogLine("a line"));

        assert_eq!(received.lock().unwrap().as_slice(), ["a line".to_owned()]);
    }
}
