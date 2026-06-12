//! Live end-to-end check against a real PTY daemon. Ignored by default.
//!
//! Run a daemon on a temp dir, then:
//!   TILLERD_DAEMON_SOCK=<dir>/daemon.sock \
//!     cargo test -p tillerd-orchestrator --test live_surface -- --ignored --nocapture

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use orchestrator::persistence::memory::InMemoryStore;
use orchestrator::persistence::{Store, SurfaceId, SurfaceKind};
use orchestrator::surface::{SurfaceEventSink, SurfaceRuntime};

struct Collect(Mutex<Vec<u8>>);

impl SurfaceEventSink for Collect {
    fn on_bytes(&self, _surface: &SurfaceId, bytes: &[u8]) {
        self.0.lock().unwrap().extend_from_slice(bytes);
    }
    fn on_status(&self, _surface: &SurfaceId, _status: &str) {}
    fn on_exit(&self, _surface: &SurfaceId, _qualifier: &str) {}
}

fn setup(sock: &str) -> (SurfaceRuntime, Arc<Collect>) {
    let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
    let sink = Arc::new(Collect(Mutex::new(Vec::new())));
    let runtime = SurfaceRuntime::new(store, sink.clone(), PathBuf::from(sock));
    (runtime, sink)
}

async fn wait_for(sink: &Collect, marker: &str) -> bool {
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if String::from_utf8_lossy(&sink.0.lock().unwrap()).contains(marker) {
            return true;
        }
    }
    false
}

fn dump(sink: &Collect) -> String {
    String::from_utf8_lossy(&sink.0.lock().unwrap()).to_string()
}

fn sock_from_env() -> String {
    std::env::var("TILLERD_DAEMON_SOCK").expect("set TILLERD_DAEMON_SOCK")
}

#[tokio::test]
#[ignore = "requires a live daemon; set TILLERD_DAEMON_SOCK"]
async fn live_terminal_streams_and_echoes_input() {
    let (runtime, sink) = setup(&sock_from_env());
    let surface = SurfaceId::from_string("live-terminal-1");
    runtime
        .launch_surface(
            surface.clone(),
            SurfaceKind::Terminal,
            None,
            None,
            None,
            "live-token".into(),
            80,
            24,
            "/tmp".into(),
        )
        .await
        .expect("open terminal against the live daemon");

    // Let the login shell start and emit its prompt.
    tokio::time::sleep(Duration::from_millis(900)).await;
    runtime
        .input(&surface, b"echo TILLERD_LIVE_OK\n")
        .await
        .expect("send input");

    let found = wait_for(&sink, "TILLERD_LIVE_OK").await;
    let out = dump(&sink);
    println!("--- live terminal output ---\n{out}\n--- end ---");
    assert!(found, "expected echoed output not seen; got:\n{out}");

    runtime.remove(&surface).await.expect("remove");
}

#[tokio::test]
#[ignore = "requires a live daemon; set TILLERD_DAEMON_SOCK"]
async fn live_resume_replays_scrollback_after_reattach() {
    let sock = sock_from_env();
    let surface = SurfaceId::from_string("live-resume-1");

    let (runtime_a, _sink_a) = setup(&sock);
    runtime_a
        .launch_surface(
            surface.clone(),
            SurfaceKind::Terminal,
            None,
            None,
            None,
            "live-token".into(),
            80,
            24,
            "/tmp".into(),
        )
        .await
        .expect("open");
    tokio::time::sleep(Duration::from_millis(900)).await;
    runtime_a
        .input(&surface, b"echo RESUME_SCROLLBACK\n")
        .await
        .expect("input");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Simulate a host restart: detach (leaving the daemon session alive), then drop the runtime.
    runtime_a.detach(&surface).await.expect("detach");
    drop(runtime_a);

    // A fresh runtime reattaches by surface id and must receive the replay paint.
    let (runtime_b, sink_b) = setup(&sock);
    runtime_b
        .resume(surface.clone())
        .await
        .expect("resume the live session");

    let painted = wait_for(&sink_b, "RESUME_SCROLLBACK").await;
    let out = dump(&sink_b);
    println!("--- resume replay paint ---\n{out}\n--- end ---");
    assert_eq!(runtime_b.proxy_count().await, 1);
    assert!(painted, "resume did not replay scrollback; got:\n{out}");

    runtime_b.remove(&surface).await.expect("remove");
}
