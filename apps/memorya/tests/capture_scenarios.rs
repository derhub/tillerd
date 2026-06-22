//! Capture scenarios: 1-to-1 mapping. Remaining in capture/queue/worker modules.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use contracts::{CorrelationId, HookEvent, HookKind, SessionId};
use memorya::capture::HookCapturer;
use memorya::hook_source::{HookSource, StubSource};
use memorya::Engram;

fn make_memorya() -> (tempfile::TempDir, Arc<Mutex<Engram>>) {
    let dir = tempfile::tempdir().unwrap();
    let e = Arc::new(Mutex::new(
        Engram::open(dir.path().join("memorya.db")).unwrap(),
    ));
    (dir, e)
}

fn prompt_event(content: &str) -> HookEvent {
    prompt_event_at_turn(content, 0)
}

fn prompt_event_at_turn(content: &str, turn: i64) -> HookEvent {
    HookEvent {
        session_id: SessionId("s1".into()),
        correlation_id: CorrelationId(format!("c{turn}")),
        ts: turn + 1,
        kind: HookKind::UserPromptSubmit {
            content: content.into(),
            turn_index: Some(turn),
        },
    }
}

// -- Hook never blocks the agent -----------------------------------------------

/// Capture is fire-and-forget: dispatching a hook event completes in bounded
/// time and does not block the calling thread. A thread that simulates an
/// "agent" invoking the capturer must return well within 1 second even when
/// the capturer performs storage.
#[test]
fn hook_never_blocks_the_agent() {
    let (_d, memorya) = make_memorya();
    let cap = HookCapturer::new(memorya);
    let event = prompt_event("test prompt that must not block");

    let start = std::time::Instant::now();
    cap.dispatch(&event).unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(500),
        "dispatch must return promptly (fire-and-forget); took {elapsed:?}"
    );
}

// -- Source is swappable for tests ---------------------------------------------

/// In production the hook source is a gate subscription; in tests it is a stub.
/// The capturer's behavior is identical regardless of the source -- this
/// verifies that `HookSource` is the only coupling point.
#[test]
fn source_is_swappable_for_tests_without_changing_capturer() {
    let events = vec![
        prompt_event_at_turn("first", 0),
        prompt_event_at_turn("second", 1),
    ];

    let (_d, memorya) = make_memorya();
    let cap = HookCapturer::new(memorya.clone());
    let mut source = StubSource::new(events);

    let mut captured = 0;
    while let Some(e) = source.next() {
        if cap.dispatch(&e).unwrap().is_some() {
            captured += 1;
        }
    }

    assert_eq!(captured, 2, "both events captured through the stub source");
    // The stub source has the same HookSource interface as GateSubscriptionSource.
    let count = memorya.lock().unwrap().active_chunk_count().unwrap();
    assert_eq!(count, 2);
}

// -- Queue survives restart (dedicated) ---------------------------------------

/// Pending embedding requests survive a process restart (db reopen). This is
/// the durability guarantee: at-least-once draining requires the queue to
/// outlive the process that enqueued it.
#[test]
fn queue_survives_restart_pending_requests_present_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memorya.db");
    {
        let memorya = Engram::open(&path).unwrap();
        // `ingest` commits the chunk and enqueues the embedding request atomically.
        memorya
            .ingest(memorya::NewChunk {
                session_id: None,
                kind: memorya::ChunkKind::Chunk,
                content: "durable content".into(),
                title: None,
                file_path: None,
                turn_index: Some(0),
                ts: 1,
            })
            .unwrap();
        let pending = memorya.pending_capture_count().unwrap();
        assert_eq!(pending, 1, "request enqueued before simulated restart");
        // Engram drops here, simulating process exit before the worker drains.
    }
    // Simulated restart: open the database anew.
    let reopened = Engram::open(&path).unwrap();
    let pending = reopened.pending_capture_count().unwrap();
    assert!(
        pending >= 1,
        "the pending request persists after reopening the database (restart simulation)"
    );
}

// -- Worker drains proactively -------------------------------------------------

/// The background worker drains pending requests without waiting for a recall
/// to be issued. This confirms proactive draining -- not lazy on read.
#[test]
fn worker_drains_proactively_without_a_recall_being_issued() {
    use memorya::worker::EmbeddingWorker;
    use std::sync::atomic::{AtomicBool, Ordering};

    let (_d, memorya) = make_memorya();
    let cap = HookCapturer::new(memorya.clone());
    cap.dispatch(&prompt_event("content to embed proactively"))
        .unwrap();

    let before = memorya.lock().unwrap().pending_capture_count().unwrap();
    assert_eq!(before, 1, "one request pending before the worker starts");

    let stop = Arc::new(AtomicBool::new(false));
    let handle = EmbeddingWorker::spawn(memorya.clone(), Duration::from_millis(10), stop.clone());

    // Wait for the worker to drain without any recall being issued.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while memorya.lock().unwrap().pending_capture_count().unwrap() > 0 {
        if std::time::Instant::now() > deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    stop.store(true, Ordering::Relaxed);
    handle.join().unwrap();

    let after = memorya.lock().unwrap().pending_capture_count().unwrap();
    assert_eq!(
        after, 0,
        "the worker drained the queue proactively without a recall being issued"
    );
}
