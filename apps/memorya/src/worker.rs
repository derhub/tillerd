//! Worker: drains capture queue, embeds chunks. Shares store via mutex.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::Engram;

/// Environment override for the drain interval, in milliseconds.
pub const DRAIN_INTERVAL_ENV: &str = "ATHING_EMBED_DRAIN_INTERVAL_MS";

/// The drain interval when the override is unset or invalid.
const DEFAULT_DRAIN_INTERVAL_MS: u64 = 5_000;

/// Requests embedded per drain cycle.
const BATCH_SIZE: i64 = 100;

/// The longest a single sleep step runs, so a stop signal is observed promptly.
const SLEEP_STEP: Duration = Duration::from_millis(25);

/// Resolve the drain interval from the environment, falling back to the default.
pub fn drain_interval() -> Duration {
    let ms = std::env::var(DRAIN_INTERVAL_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_DRAIN_INTERVAL_MS);
    Duration::from_millis(ms)
}

/// Spawns and owns the background embedding thread.
pub struct EmbeddingWorker;

impl EmbeddingWorker {
    /// Spawn the drain loop. It reclaims requests abandoned by a previous run,
    /// then drains and embeds on `interval` until `stop` is set.
    pub fn spawn(
        memorya: Arc<Mutex<Engram>>,
        interval: Duration,
        stop: Arc<AtomicBool>,
    ) -> JoinHandle<()> {
        thread::spawn(move || run(&memorya, interval, &stop))
    }
}

fn run(memorya: &Arc<Mutex<Engram>>, interval: Duration, stop: &AtomicBool) {
    match memorya.lock() {
        Ok(memorya) => {
            if let Err(e) = memorya.reclaim_stale_captures() {
                eprintln!("memorya embedding worker: reclaim failed: {e}");
            }
        }
        Err(_) => return,
    }

    while !stop.load(Ordering::Relaxed) {
        match memorya.lock() {
            Ok(memorya) => {
                if let Err(e) = memorya.drain_and_embed_captures(BATCH_SIZE) {
                    eprintln!("memorya embedding worker: drain failed: {e}");
                }
            }
            // A poisoned mutex means another thread panicked mid-write; there is
            // nothing the worker can safely do, so it exits.
            Err(_) => return,
        }
        sleep_until_due(interval, stop);
    }
}

fn sleep_until_due(interval: Duration, stop: &AtomicBool) {
    let step = interval.min(SLEEP_STEP).max(Duration::from_millis(1));
    let mut elapsed = Duration::ZERO;
    while elapsed < interval {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        thread::sleep(step);
        elapsed += step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{embed, ChunkKind, Engram, NewChunk, RecallResult};
    use std::time::Instant;

    fn shared() -> (tempfile::TempDir, Arc<Mutex<Engram>>) {
        let dir = tempfile::tempdir().unwrap();
        let memorya = Arc::new(Mutex::new(
            Engram::open(dir.path().join("memorya.db")).unwrap(),
        ));
        (dir, memorya)
    }

    fn ingest_doc(memorya: &Arc<Mutex<Engram>>, path: &str, body: &str) -> i64 {
        memorya
            .lock()
            .unwrap()
            .ingest(NewChunk {
                session_id: None,
                kind: ChunkKind::Doc,
                content: body.into(),
                title: Some("doc".into()),
                file_path: Some(path.into()),
                turn_index: None,
                ts: 0,
            })
            .unwrap()
            .unwrap()
    }

    fn pending(memorya: &Arc<Mutex<Engram>>) -> i64 {
        memorya.lock().unwrap().pending_capture_count().unwrap()
    }

    fn total_queue_rows(memorya: &Arc<Mutex<Engram>>) -> i64 {
        memorya
            .lock()
            .unwrap()
            .store()
            .conn()
            .query_row("SELECT COUNT(*) FROM capture_queue", [], |r| r.get(0))
            .unwrap()
    }

    fn max_attempts(memorya: &Arc<Mutex<Engram>>) -> i64 {
        memorya
            .lock()
            .unwrap()
            .store()
            .conn()
            .query_row(
                "SELECT COALESCE(MAX(attempts), 0) FROM capture_queue",
                [],
                |r| r.get(0),
            )
            .unwrap()
    }

    fn wait_until(cond: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if cond() {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }
        cond()
    }

    /// An embedder that always panics, to exercise the worker's failure handling.
    struct PanicEmbedder;
    impl embed::Embedder for PanicEmbedder {
        fn model_id(&self) -> &str {
            "panic-model"
        }
        fn dim(&self) -> usize {
            8
        }
        fn embed(&self, _text: &str) -> Vec<f32> {
            panic!("embed boom");
        }
    }

    #[test]
    fn worker_drains_queue_on_configured_interval() {
        let (_d, memorya) = shared();
        ingest_doc(&memorya, "/p/a.md", "alpha content");
        let stop = Arc::new(AtomicBool::new(false));
        let handle =
            EmbeddingWorker::spawn(memorya.clone(), Duration::from_millis(5), stop.clone());

        let drained = wait_until(|| pending(&memorya) == 0);

        stop.store(true, Ordering::Relaxed);
        handle.join().unwrap();
        assert!(drained, "the worker drained the queue on its interval");
    }

    #[test]
    fn worker_embeds_pending_chunks_then_recall_finds_them() {
        let (_d, memorya) = shared();
        ingest_doc(&memorya, "/p/db.md", "the project stores data in postgres");
        let stop = Arc::new(AtomicBool::new(false));
        let handle =
            EmbeddingWorker::spawn(memorya.clone(), Duration::from_millis(5), stop.clone());

        let drained = wait_until(|| pending(&memorya) == 0);
        stop.store(true, Ordering::Relaxed);
        handle.join().unwrap();

        assert!(drained, "the worker embedded the pending chunk");
        let result = memorya
            .lock()
            .unwrap()
            .recall("postgres data store", 10)
            .unwrap();
        assert!(matches!(result, RecallResult::Found { .. }));
    }

    #[test]
    fn worker_stops_on_signal() {
        let (_d, memorya) = shared();
        let stop = Arc::new(AtomicBool::new(false));
        let handle = EmbeddingWorker::spawn(memorya, Duration::from_millis(5), stop.clone());

        let start = Instant::now();
        stop.store(true, Ordering::Relaxed);
        handle.join().unwrap();

        assert!(
            start.elapsed() < Duration::from_secs(2),
            "the worker stopped promptly"
        );
    }

    #[test]
    fn worker_logs_and_reschedules_on_embed_failure_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let memorya = Arc::new(Mutex::new(
            Engram::open_with(dir.path().join("memorya.db"), Box::new(PanicEmbedder)).unwrap(),
        ));
        ingest_doc(&memorya, "/p/x.md", "this embedding will panic");
        let stop = Arc::new(AtomicBool::new(false));
        let handle =
            EmbeddingWorker::spawn(memorya.clone(), Duration::from_millis(50), stop.clone());

        let rescheduled = wait_until(|| max_attempts(&memorya) >= 1);

        stop.store(true, Ordering::Relaxed);
        handle.join().unwrap();
        assert!(
            rescheduled,
            "a failed embedding records an attempt and reschedules"
        );
        assert!(pending(&memorya) >= 1, "the failed request stays pending");
    }

    #[test]
    fn worker_reclaims_stale_requests_on_startup() {
        let (_d, memorya) = shared();
        ingest_doc(&memorya, "/p/s.md", "abandoned content");
        // Drain marks it 'processing' but it is never embedded: a crashed run.
        crate::queue::drain_batch(memorya.lock().unwrap().store(), 10).unwrap();
        assert_eq!(total_queue_rows(&memorya), 1, "the abandoned row remains");

        let stop = Arc::new(AtomicBool::new(false));
        let handle =
            EmbeddingWorker::spawn(memorya.clone(), Duration::from_millis(5), stop.clone());

        let cleared = wait_until(|| total_queue_rows(&memorya) == 0);

        stop.store(true, Ordering::Relaxed);
        handle.join().unwrap();
        assert!(
            cleared,
            "startup reclaim re-queued and drained the abandoned request"
        );
    }

    #[test]
    fn background_worker_drains_without_waiting_for_a_recall_request() {
        let (_d, memorya) = shared();
        ingest_doc(&memorya, "/p/bg.md", "background content");
        let stop = Arc::new(AtomicBool::new(false));
        let handle =
            EmbeddingWorker::spawn(memorya.clone(), Duration::from_millis(5), stop.clone());

        // No recall is ever issued; the queue must still drain proactively.
        let drained = wait_until(|| pending(&memorya) == 0);

        stop.store(true, Ordering::Relaxed);
        handle.join().unwrap();
        assert!(drained, "embedding is proactive, not triggered by a recall");
    }
}
