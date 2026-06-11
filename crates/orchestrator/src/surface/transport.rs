use std::path::{Path, PathBuf};
use std::sync::Arc;

use contracts::SessionId;
use daemon_pty_client::{
    decode_session_frame, encode_ack, encode_hello, encode_input, encode_kill, encode_resize,
    encode_spawn, encode_subscribe, encode_unsubscribe, FrameDecoder, RawFrame, SessionFrame,
    SpawnParams,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::UnixStream;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("connect error: {0}")]
    Connect(#[from] std::io::Error),

    #[error("handshake error: {0}")]
    Handshake(String),

    #[error("version mismatch: daemon returned version {found}")]
    VersionMismatch { found: u32 },

    #[error("connection closed")]
    Closed,
}

pub struct DaemonConnection {
    write: Arc<Mutex<OwnedWriteHalf>>,
}

impl std::fmt::Debug for DaemonConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DaemonConnection").finish_non_exhaustive()
    }
}

impl DaemonConnection {
    pub async fn connect(
        socket_path: &Path,
    ) -> Result<(Self, mpsc::Receiver<SessionFrame>), TransportError> {
        let stream = UnixStream::connect(socket_path).await?;
        let (mut read_half, write_half) = stream.into_split();

        let write = Arc::new(Mutex::new(write_half));

        {
            let hello = encode_hello(&["snapshot"]);
            let mut guard = write.lock().await;
            guard.write_all(&hello).await?;
        }

        let mut decoder = FrameDecoder::new();
        let first_raw = read_first_frame(&mut read_half, &mut decoder).await?;

        match decode_session_frame(&first_raw) {
            Some(SessionFrame::HelloAck { version, .. }) => {
                if version != 1 {
                    return Err(TransportError::VersionMismatch { found: version });
                }
            }
            Some(other) => {
                return Err(TransportError::Handshake(format!(
                    "expected hello-ack, got {:?}",
                    other
                )));
            }
            None => {
                return Err(TransportError::Handshake(
                    "received undecodable frame during handshake".to_string(),
                ));
            }
        }

        let (tx, rx) = mpsc::channel::<SessionFrame>(256);
        tokio::spawn(read_loop(read_half, decoder, tx));

        Ok((DaemonConnection { write }, rx))
    }

    pub async fn spawn(&self, params: &SpawnParams<'_>) -> Result<(), TransportError> {
        self.write_frame(encode_spawn(params)).await
    }

    pub async fn subscribe(&self, id: &SessionId) -> Result<(), TransportError> {
        self.write_frame(encode_subscribe(id)).await
    }

    pub async fn input(&self, id: &SessionId, bytes: &[u8]) -> Result<(), TransportError> {
        self.write_frame(encode_input(id, bytes)).await
    }

    pub async fn resize(
        &self,
        id: &SessionId,
        cols: u16,
        rows: u16,
    ) -> Result<(), TransportError> {
        self.write_frame(encode_resize(id, cols, rows)).await
    }

    pub async fn ack(&self, id: &SessionId, bytes: i64) -> Result<(), TransportError> {
        self.write_frame(encode_ack(id, bytes)).await
    }

    pub async fn unsubscribe(&self, id: &SessionId) -> Result<(), TransportError> {
        self.write_frame(encode_unsubscribe(id)).await
    }

    pub async fn kill(&self, id: &SessionId) -> Result<(), TransportError> {
        self.write_frame(encode_kill(id)).await
    }

    async fn write_frame(&self, frame: Vec<u8>) -> Result<(), TransportError> {
        let mut guard = self.write.lock().await;
        guard
            .write_all(&frame)
            .await
            .map_err(|_| TransportError::Closed)
    }
}

pub fn default_daemon_socket() -> PathBuf {
    service_host::paths::resolve_base_dir(std::env::var("TILLERD_DIR").ok().as_deref())
        .join("daemon.sock")
}

async fn read_first_frame(
    read_half: &mut tokio::net::unix::OwnedReadHalf,
    decoder: &mut FrameDecoder,
) -> Result<RawFrame, TransportError> {
    let mut buf = vec![0u8; 4096];
    loop {
        let n = read_half.read(&mut buf).await?;
        if n == 0 {
            return Err(TransportError::Handshake(
                "connection closed before hello-ack".to_string(),
            ));
        }
        let mut frames = decoder.push(&buf[..n]);
        if !frames.is_empty() {
            return Ok(frames.remove(0));
        }
    }
}

async fn read_loop(
    mut read_half: tokio::net::unix::OwnedReadHalf,
    mut decoder: FrameDecoder,
    tx: mpsc::Sender<SessionFrame>,
) {
    let mut buf = vec![0u8; 4096];
    loop {
        match read_half.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let frames = decoder.push(&buf[..n]);
                for raw in frames {
                    if let Some(frame) = decode_session_frame(&raw) {
                        if tx.send(frame).await.is_err() {
                            return;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use daemon_pty_client::encode_frame;
    use tokio::net::UnixListener;

    async fn fake_daemon<F, Fut>(dir: &std::path::Path, fake: F) -> PathBuf
    where
        F: FnOnce(tokio::net::UnixStream) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let path = dir.join("daemon.sock");
        let listener = UnixListener::bind(&path).expect("bind");
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            fake(stream).await;
        });
        path
    }

    #[tokio::test]
    async fn connect_handshakes_and_spawn_delivers_spawn_ack() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sock = fake_daemon(dir.path(), |stream| async move {
            let (mut rx, mut tx) = stream.into_split();
            let mut buf = vec![0u8; 512];

            let mut decoder = FrameDecoder::new();
            let hello_raw = loop {
                let n = rx.read(&mut buf).await.expect("read");
                assert!(n > 0, "connection closed during hello");
                let mut frames = decoder.push(&buf[..n]);
                if !frames.is_empty() {
                    break frames.remove(0);
                }
            };
            let meta: serde_json::Value =
                serde_json::from_slice(&hello_raw.meta).expect("hello meta json");
            assert_eq!(meta["type"], "hello");

            let ack = encode_frame(
                br#"{"type":"hello-ack","version":1,"daemonVersion":"0.0.0","capabilities":["snapshot"]}"#,
                None,
            );
            tx.write_all(&ack).await.expect("write hello-ack");

            let mut decoder2 = FrameDecoder::new();
            let spawn_raw = loop {
                let n = rx.read(&mut buf).await.expect("read spawn");
                assert!(n > 0, "connection closed during spawn");
                let mut frames = decoder2.push(&buf[..n]);
                if !frames.is_empty() {
                    break frames.remove(0);
                }
            };
            let smeta: serde_json::Value =
                serde_json::from_slice(&spawn_raw.meta).expect("spawn meta json");
            assert_eq!(smeta["type"], "spawn");

            let spawn_ack = encode_frame(
                br#"{"type":"spawn-ack","sessionId":"test-sess","pid":1234}"#,
                None,
            );
            tx.write_all(&spawn_ack).await.expect("write spawn-ack");
        })
        .await;

        let (conn, mut receiver) = DaemonConnection::connect(&sock)
            .await
            .expect("connect should succeed");

        let params = SpawnParams {
            session_id: &SessionId("test-sess".to_string()),
            token: "tok",
            cols: 80,
            rows: 24,
            cwd: "/tmp",
            command: None,
            args: &[],
            env: None,
            resume: None,
        };
        conn.spawn(&params).await.expect("spawn should succeed");

        let frame = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            receiver.recv(),
        )
        .await
        .expect("timed out waiting for spawn-ack")
        .expect("channel closed before spawn-ack");

        assert!(
            matches!(frame, SessionFrame::SpawnAck { .. }),
            "expected SpawnAck, got {:?}",
            frame
        );
    }

    #[tokio::test]
    async fn connect_returns_version_mismatch_on_wrong_version() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sock = fake_daemon(dir.path(), |stream| async move {
            let (_rx, mut tx) = stream.into_split();
            let bad_ack = encode_frame(
                br#"{"type":"hello-ack","version":99,"daemonVersion":"9.9.9","capabilities":[]}"#,
                None,
            );
            tx.write_all(&bad_ack).await.expect("write bad ack");
        })
        .await;

        let result = DaemonConnection::connect(&sock).await;
        assert!(
            matches!(result, Err(TransportError::VersionMismatch { found: 99 })),
            "expected VersionMismatch(99), got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn connect_returns_handshake_error_on_unexpected_first_frame() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sock = fake_daemon(dir.path(), |stream| async move {
            let (_rx, mut tx) = stream.into_split();
            let wrong = encode_frame(
                br#"{"type":"spawn-ack","sessionId":"s","pid":1}"#,
                None,
            );
            tx.write_all(&wrong).await.expect("write wrong frame");
        })
        .await;

        let result = DaemonConnection::connect(&sock).await;
        assert!(
            matches!(result, Err(TransportError::Handshake(_))),
            "expected Handshake error, got {:?}",
            result
        );
    }
}
