use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::UnixStream;
use tokio::sync::Mutex;

use tillerd_paths::daemon_socket;

/// Event emitted to the renderer when the daemon connection drops unexpectedly.
pub const DAEMON_LOST_EVENT: &str = "daemon-lost";

/// Unit payload for the daemon-lost event (connection dropped unexpectedly).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct DaemonLost;

/// The byte bridge to the daemon's Unix socket. Forwards renderer bytes verbatim and streams
/// daemon output back over a Channel -- it never parses a frame.
#[derive(Default)]
pub struct BridgeState {
    writer: Arc<Mutex<Option<OwnedWriteHalf>>>,
    closing: Arc<AtomicBool>,
}

#[tauri::command]
#[specta::specta]
pub async fn daemon_connect(
    app: AppHandle,
    channel: Channel<Vec<u8>>,
    state: State<'_, BridgeState>,
) -> Result<(), String> {
    let sock = daemon_socket();
    let stream = UnixStream::connect(&sock)
        .await
        .map_err(|e| format!("daemon connect {}: {}", sock.display(), e))?;
    let (mut read_half, write_half) = stream.into_split();

    state.closing.store(false, Ordering::SeqCst);
    *state.writer.lock().await = Some(write_half);

    let writer = state.writer.clone();
    let closing = state.closing.clone();
    tauri::async_runtime::spawn(async move {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            match read_half.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    // Channel preserves order; the daemon flow-control credit/ack loop survives
                    // the hop as long as no bytes are dropped/reordered.
                    if channel.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
        *writer.lock().await = None;
        // An uninitiated drop is a lost connection -- surface it as a typed error.
        if !closing.load(Ordering::SeqCst) {
            let _ = app.emit(DAEMON_LOST_EVENT, ());
        }
    });

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn daemon_send(bytes: Vec<u8>, state: State<'_, BridgeState>) -> Result<(), String> {
    let mut guard = state.writer.lock().await;
    match guard.as_mut() {
        Some(write) => write.write_all(&bytes).await.map_err(|e| e.to_string()),
        None => Err("daemon not connected".into()),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn daemon_disconnect(state: State<'_, BridgeState>) -> Result<(), String> {
    state.closing.store(true, Ordering::SeqCst);
    if let Some(mut write) = state.writer.lock().await.take() {
        let _ = write.shutdown().await;
    }
    Ok(())
}
