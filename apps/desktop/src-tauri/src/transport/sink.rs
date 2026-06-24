//! The tauri implementation of the surface output sink. Bytes for each surface are
//! written to a per-surface `tauri::ipc::Channel<Vec<u8>>` the renderer registers;
//! status/exit/error are forwarded as tauri events. A future web transport
//! implements the same `SurfaceSink` port with SSE/WebSocket.
//!
//! Keystroke input never flows through this sink -- it carries daemon -> renderer
//! output only (see the off-bus input endpoints), so no payload is ever logged here.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use orchestrator::app::surface::{SurfaceEvent, SurfaceSink};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};

/// The per-surface output channels, keyed by surface id. The renderer creates a
/// `Channel` per surface and registers it (at spawn/attach); the sink looks it up by
/// id to deliver bytes. Shared (`Arc`) so the registering command and the sink see the
/// same map.
pub type SurfaceChannels = Arc<Mutex<HashMap<String, tauri::ipc::Channel<Vec<u8>>>>>;

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

/// Bridges runtime output to the renderer over tauri IPC. Generic over the runtime so
/// it works under both `Wry` and the `tauri::test` mock runtime.
pub struct ChannelSink<R: Runtime> {
    channels: SurfaceChannels,
    app: AppHandle<R>,
}

impl<R: Runtime> ChannelSink<R> {
    /// Build a sink over a shared channel registry and the app handle it emits events
    /// through.
    pub fn new(channels: SurfaceChannels, app: AppHandle<R>) -> Self {
        Self { channels, app }
    }
}

impl<R: Runtime> SurfaceSink for ChannelSink<R> {
    fn emit(&self, surface: &str, event: &SurfaceEvent<'_>) {
        match event {
            SurfaceEvent::Bytes(bytes) => {
                deliver(&self.channels, surface, bytes);
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
                self.channels
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(surface);
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

/// Deliver bytes to a surface's registered output channel, if any. A surface with no
/// registered channel (never attached, or already detached) silently drops its bytes --
/// there is nowhere to stream.
fn deliver(channels: &SurfaceChannels, surface: &str, bytes: &[u8]) {
    let channels = channels.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(channel) = channels.get(surface) {
        let _ = channel.send(bytes.to_vec());
    }
}

/// Register a surface's output channel so the sink can deliver its bytes. The renderer
/// calls this (via a thin command) before attach/spawn so no initial output is lost.
pub fn register_channel(
    channels: &SurfaceChannels,
    surface: &str,
    channel: tauri::ipc::Channel<Vec<u8>>,
) {
    channels
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(surface.to_owned(), channel);
}

/// Drop a surface's output channel (on detach/close); the sink then has nowhere to
/// deliver, so its bytes are discarded.
pub fn unregister_channel(channels: &SurfaceChannels, surface: &str) {
    channels
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(surface);
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// Captured payloads delivered to a recording channel, shared with the test.
    type Recorded = Arc<Mutex<Vec<Vec<u8>>>>;

    /// A `Channel` that records every payload it receives, so a test can assert what
    /// the sink delivered without a running tauri app.
    fn recording_channel() -> (tauri::ipc::Channel<Vec<u8>>, Recorded) {
        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = received.clone();
        let channel = tauri::ipc::Channel::new(move |body| {
            // `Channel::send(Vec<u8>)` serializes through the blanket `Serialize` path,
            // so the body is a JSON array of byte values.
            if let tauri::ipc::InvokeResponseBody::Json(json) = body {
                let bytes: Vec<u8> = serde_json::from_str(&json).unwrap_or_default();
                sink.lock().unwrap().push(bytes);
            }
            Ok(())
        });
        (channel, received)
    }

    fn channels() -> SurfaceChannels {
        Arc::new(Mutex::new(HashMap::new()))
    }

    // A registered channel receives the bytes delivered for its surface.
    #[test]
    fn deliver_sends_bytes_to_the_registered_channel() {
        let channels = channels();
        let (channel, received) = recording_channel();
        register_channel(&channels, "surf-1", channel);

        deliver(&channels, "surf-1", b"hello");

        assert_eq!(received.lock().unwrap().as_slice(), [b"hello".to_vec()]);
    }

    // A surface with no registered channel silently drops its bytes.
    #[test]
    fn deliver_drops_bytes_for_an_unregistered_surface() {
        let channels = channels();
        // no panic, no delivery
        deliver(&channels, "surf-unknown", b"hello");
    }

    // After unregister, further bytes are dropped -- the stream went quiet.
    #[test]
    fn deliver_drops_bytes_after_unregister() {
        let channels = channels();
        let (channel, received) = recording_channel();
        register_channel(&channels, "surf-1", channel);
        unregister_channel(&channels, "surf-1");

        deliver(&channels, "surf-1", b"after-detach");

        assert!(received.lock().unwrap().is_empty());
    }
}
