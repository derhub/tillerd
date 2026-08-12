//! The log-follow pump: watches the runtime logs directory with `notify` and, on
//! every append to a `.log` file, reads the newly-appended lines from the last
//! tracked byte offset and fans them out -- borrowed and key-scoped by service --
//! to the registered [`LogSink`]s.
//!
//! The file read is offloaded to a blocking pool by `shared::fs::tail` (windowed
//! `spawn_blocking`), so the watcher's async task is never blocked on disk I/O.
//! Per-file byte offsets are tracked so only appended bytes are read and emitted;
//! a truncation (rotation) resets that file's offset to zero.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use notify::{Event, RecursiveMode, Watcher};

use crate::shared::bus::Registry;
use crate::shared::domain_channel::{DomainChannelEvent, DomainChannelSink};
use crate::shared::fs;

/// Window read per append event. A single tracing record line is far smaller; a
/// burst between events is drained over successive reads from the carried offset.
const READ_WINDOW: u64 = 1 << 20;

/// Tracks per-file byte offsets and dispatches appended lines to the domain channel
/// registry, keyed by the service prefix derived from each file name.
///
/// `read_appended` is the deterministic core: given a path it has a tracked
/// offset for, it reads the complete lines added since that offset, advances the
/// offset, and emits each line under the file's service key. The `notify` loop in
/// [`run`] only decides *when* to call it.
pub struct LogFollower {
    dir: PathBuf,
    registry: Arc<Registry<dyn DomainChannelSink>>,
    offsets: HashMap<PathBuf, u64>,
}

/// The service key for a `.log` file is its name up to the first `.` --
/// `tillerd-daemon.2026-06-25.log` -> `tillerd-daemon`. Non-`.log` files yield
/// `None` and are ignored.
pub(crate) fn service_of(path: &Path) -> Option<String> {
    if path.extension().and_then(|x| x.to_str()) != Some("log") {
        return None;
    }
    let name = path.file_name()?.to_str()?;
    let prefix = name.split('.').next()?;
    if prefix.is_empty() {
        return None;
    }
    Some(prefix.to_owned())
}

impl LogFollower {
    pub fn new(dir: PathBuf, registry: Arc<Registry<dyn DomainChannelSink>>) -> Self {
        Self {
            dir,
            registry,
            offsets: HashMap::new(),
        }
    }

    /// Read the lines appended to `path` since its tracked offset and emit each
    /// under the file's service key, then advance the offset. No-op for a
    /// non-`.log` path. A file shorter than its tracked offset (rotated/truncated)
    /// is re-read from zero.
    pub async fn read_appended(&mut self, path: &Path) {
        let Some(service) = service_of(path) else {
            return;
        };
        let from = self.offsets.get(path).copied().unwrap_or(0);
        let from = match tokio::fs::metadata(path).await {
            Ok(m) if m.len() < from => 0,
            _ => from,
        };
        let Ok(tail) = fs::tail(path, from, READ_WINDOW, false).await else {
            return;
        };
        for line in &tail.lines {
            let event = DomainChannelEvent::Bytes(line.as_bytes());
            self.registry
                .dispatch(&format!("logs://{}", service), |s| s.emit(&event));
        }
        self.offsets.insert(path.to_owned(), tail.end);
    }

    async fn read_event(&mut self, event: &Event) {
        for path in &event.paths {
            if path == &self.dir {
                self.read_directory().await;
            } else {
                self.read_appended(path).await;
            }
        }
    }

    async fn read_directory(&mut self) {
        let Ok(mut entries) = tokio::fs::read_dir(&self.dir).await else {
            return;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            self.read_appended(&entry.path()).await;
        }
    }

    /// Seed the tracked offset of every existing `.log` file to its current end,
    /// so a fresh subscription streams only lines appended after it begins (not
    /// the whole backlog -- callers backfill via `TailLog`).
    pub async fn seed_offsets(&mut self) {
        let Ok(mut rd) = tokio::fs::read_dir(&self.dir).await else {
            return;
        };
        while let Ok(Some(entry)) = rd.next_entry().await {
            let path = entry.path();
            if service_of(&path).is_none() {
                continue;
            }
            let end = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
            self.offsets.insert(path, end);
        }
    }

    /// Watch the logs directory and drain appended lines on every change. Seeds
    /// offsets to the current ends first, then installs a `notify` watcher whose
    /// events arrive on an unbounded channel and reads each touched path. Returns
    /// only if the watcher cannot be installed or its channel closes.
    pub async fn run(mut self) {
        self.seed_offsets().await;

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Event>();
        let mut watcher = match notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        }) {
            Ok(w) => w,
            Err(error) => {
                tracing::error!(error.type = "log.watch", %error, "log follower watcher init failed");
                return;
            }
        };
        if let Err(error) = watcher.watch(&self.dir, RecursiveMode::NonRecursive) {
            tracing::error!(error.type = "log.watch", %error, "log follower watch failed");
            return;
        }

        while let Some(event) = rx.recv().await {
            self.read_event(&event).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    type Captured = Arc<Mutex<Vec<(String, String)>>>;

    /// A sink that records `(service, line)` pairs it receives.
    struct Recorder {
        captured: Captured,
        service: String,
    }

    impl DomainChannelSink for Recorder {
        fn emit(&self, event: &DomainChannelEvent<'_>) {
            if let DomainChannelEvent::Bytes(bytes) = event {
                let line = std::str::from_utf8(bytes).unwrap().to_owned();
                self.captured
                    .lock()
                    .unwrap()
                    .push((self.service.clone(), line));
            }
        }
    }

    fn follower_with_sink(dir: &Path, key: &str) -> (LogFollower, Captured) {
        let log: Captured = Arc::default();
        let registry: Arc<Registry<dyn DomainChannelSink>> = Arc::default();
        registry.register(
            &format!("logs://{}", key),
            Arc::new(Recorder {
                captured: Arc::clone(&log),
                service: key.to_owned(),
            }),
        );
        (LogFollower::new(dir.to_owned(), registry), log)
    }

    fn append(path: &Path, text: &str) {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        f.write_all(text.as_bytes()).unwrap();
        f.flush().unwrap();
    }

    #[test]
    fn service_of_takes_the_prefix_before_the_first_dot() {
        let p = Path::new("/x/logs/tillerd-daemon.2026-06-25.log");
        assert_eq!(service_of(p).as_deref(), Some("tillerd-daemon"));
    }

    #[test]
    fn service_of_ignores_non_log_files() {
        assert_eq!(service_of(Path::new("/x/logs/notes.txt")), None);
    }

    #[tokio::test]
    async fn read_appended_emits_each_new_line_under_the_service_key() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("tillerd-daemon.2026-06-25.log");
        append(&file, "one\ntwo\n");

        let (mut follower, log) = follower_with_sink(dir.path(), "tillerd-daemon");
        follower.read_appended(&file).await;

        assert_eq!(
            log.lock().unwrap().as_slice(),
            [
                ("tillerd-daemon".to_owned(), "one".to_owned()),
                ("tillerd-daemon".to_owned(), "two".to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn a_directory_event_rescans_and_emits_appended_lines() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("tillerd-daemon.2026-06-25.log");
        append(&file, "line\n");

        let (mut follower, log) = follower_with_sink(dir.path(), "tillerd-daemon");
        follower
            .read_event(&Event::new(notify::EventKind::Any).add_path(dir.path().to_owned()))
            .await;

        assert_eq!(
            log.lock().unwrap().as_slice(),
            [("tillerd-daemon".to_owned(), "line".to_owned())]
        );
    }

    #[tokio::test]
    async fn read_appended_only_emits_lines_added_since_the_last_offset() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("tillerd-daemon.2026-06-25.log");
        append(&file, "one\n");

        let (mut follower, log) = follower_with_sink(dir.path(), "tillerd-daemon");
        follower.read_appended(&file).await;
        log.lock().unwrap().clear();

        append(&file, "two\n");
        follower.read_appended(&file).await;

        assert_eq!(
            log.lock().unwrap().as_slice(),
            [("tillerd-daemon".to_owned(), "two".to_owned())]
        );
    }

    #[tokio::test]
    async fn a_line_for_another_service_does_not_reach_the_sink() {
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("tillerd-gate.2026-06-25.log");
        append(&other, "gate-line\n");

        // The sink is registered under `tillerd-daemon`; a gate append must not reach it.
        let (mut follower, log) = follower_with_sink(dir.path(), "tillerd-daemon");
        follower.read_appended(&other).await;

        assert!(log.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn seed_offsets_skips_the_existing_backlog() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("tillerd-daemon.2026-06-25.log");
        append(&file, "old\n");

        let (mut follower, log) = follower_with_sink(dir.path(), "tillerd-daemon");
        follower.seed_offsets().await;
        follower.read_appended(&file).await;

        assert!(log.lock().unwrap().is_empty());

        append(&file, "new\n");
        follower.read_appended(&file).await;
        assert_eq!(
            log.lock().unwrap().as_slice(),
            [("tillerd-daemon".to_owned(), "new".to_owned())]
        );
    }

    #[tokio::test]
    async fn a_truncated_file_is_re_read_from_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("tillerd-daemon.2026-06-25.log");
        append(&file, "first\nsecond\n");

        let (mut follower, log) = follower_with_sink(dir.path(), "tillerd-daemon");
        follower.read_appended(&file).await;
        log.lock().unwrap().clear();

        std::fs::write(&file, "rotated\n").unwrap();
        follower.read_appended(&file).await;

        assert_eq!(
            log.lock().unwrap().as_slice(),
            [("tillerd-daemon".to_owned(), "rotated".to_owned())]
        );
    }
}
