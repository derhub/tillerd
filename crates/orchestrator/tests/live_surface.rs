//! Live end-to-end check against a real PTY daemon. Ignored by default.
//!
//! Run a daemon on a temp dir, then:
//!   TILLERD_DAEMON_SOCK=<dir>/daemon.sock \
//!     cargo test -p tillerd-orchestrator --test live_surface -- --ignored --nocapture

use std::sync::{Arc, Mutex};
use std::time::Duration;

use orchestrator::persistence::memory::InMemoryStore;
use orchestrator::persistence::{Store, SurfaceId};
use orchestrator::surface::{SurfaceEventSink, SurfaceRuntime};

struct Collect(Mutex<Vec<u8>>);

impl SurfaceEventSink for Collect {
    fn on_bytes(&self, _surface: &SurfaceId, bytes: &[u8]) {
        self.0.lock().unwrap().extend_from_slice(bytes);
    }
    fn on_status(&self, _surface: &SurfaceId, _status: &str) {}
    fn on_exit(&self, _surface: &SurfaceId, _qualifier: &str) {}
}

#[tokio::test]
#[ignore = "requires a live daemon; set TILLERD_DAEMON_SOCK"]
async fn live_terminal_streams_and_echoes_input() {
    let sock = std::env::var("TILLERD_DAEMON_SOCK").expect("set TILLERD_DAEMON_SOCK");

    let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
    let sink = Arc::new(Collect(Mutex::new(Vec::new())));
    let runtime = SurfaceRuntime::new(store, sink.clone(), sock.into());

    let surface = SurfaceId::from_string("live-terminal-1");
    runtime
        .open_terminal(surface.clone(), "live-token".into(), 80, 24, "/tmp".into())
        .await
        .expect("open terminal against the live daemon");

    // Let the login shell start and emit its prompt.
    tokio::time::sleep(Duration::from_millis(900)).await;

    runtime
        .input(&surface, b"echo TILLERD_LIVE_OK\n")
        .await
        .expect("send input");

    let mut found = false;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if String::from_utf8_lossy(&sink.0.lock().unwrap()).contains("TILLERD_LIVE_OK") {
            found = true;
            break;
        }
    }

    let dump = String::from_utf8_lossy(&sink.0.lock().unwrap()).to_string();
    println!("--- live terminal output ---\n{dump}\n--- end ---");
    assert!(found, "expected echoed output not seen; got:\n{dump}");

    runtime.remove(&surface).await.expect("remove");
}

#[tokio::test]
#[ignore = "requires a live daemon; set TILLERD_DAEMON_SOCK"]
async fn live_resume_replays_scrollback_after_reattach() {
    let sock = std::env::var("TILLERD_DAEMON_SOCK").expect("set TILLERD_DAEMON_SOCK");
    let surface = SurfaceId::from_string("live-resume-1");

    // First runtime opens the surface and writes a marker.
    let store_a: Arc<dyn Store> = Arc::new(InMemoryStore::new());
    let sink_a = Arc::new(Collect(Mutex::new(Vec::new())));
    let runtime_a = SurfaceRuntime::new(store_a, sink_a, sock.clone().into());
    runtime_a
        .open_terminal(surface.clone(), "live-token".into(), 80, 24, "/tmp".into())
        .await
        .expect("open");
    tokio::time::sleep(Duration::from_millis(900)).await;
    runtime_a
        .input(&surface, b"echo RESUME_SCROLLBACK\n")
        .await
        .expect("input");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Simulate a host restart: detach (leaving the daemon session alive), drop it.
    runtime_a.detach(&surface).await.expect("detach");
    drop(runtime_a);

    // A fresh runtime reattaches by surface id and must receive the replay paint.
    let store_b: Arc<dyn Store> = Arc::new(InMemoryStore::new());
    let sink_b = Arc::new(Collect(Mutex::new(Vec::new())));
    let runtime_b = SurfaceRuntime::new(store_b, sink_b.clone(), sock.into());
    runtime_b
        .resume(surface.clone())
        .await
        .expect("resume the live session");

    let mut painted = false;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if String::from_utf8_lossy(&sink_b.0.lock().unwrap()).contains("RESUME_SCROLLBACK") {
            painted = true;
            break;
        }
    }

    let dump = String::from_utf8_lossy(&sink_b.0.lock().unwrap()).to_string();
    println!("--- resume replay paint ---\n{dump}\n--- end ---");
    assert_eq!(runtime_b.proxy_count().await, 1);
    assert!(painted, "resume did not replay scrollback; got:\n{dump}");

    runtime_b.remove(&surface).await.expect("remove");
}
