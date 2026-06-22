//! In-memory runtime for tests: no socket, no daemon. Tracks which surfaces it
//! considers running and records the calls made against it so app-layer tests can
//! assert the side-effect shape (persist intent -> effect -> record -> reconcile)
//! without a live daemon.

use std::collections::HashSet;
use std::sync::Mutex;

use tokio::sync::mpsc;

use super::{Output, SpawnRequest, SurfaceOutput};
use crate::entities::SurfaceId;
use crate::shared::{Error, Result};

/// A recorded runtime interaction, in call order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeCall {
    Spawn(SurfaceId),
    Stop(SurfaceId),
    Close(SurfaceId),
    Input {
        surface: SurfaceId,
        bytes: Vec<u8>,
    },
    Resize {
        surface: SurfaceId,
        cols: u16,
        rows: u16,
    },
    Attach(SurfaceId),
    Detach(SurfaceId),
    List,
}

#[derive(Default)]
struct State {
    running: HashSet<SurfaceId>,
    calls: Vec<RuntimeCall>,
}

/// An in-memory runtime fake. `spawn` marks a surface running; `stop`/`close` mark
/// it not running; `list` returns the running set. `fail_spawn` makes the next
/// `spawn` error (to drive the failed-spawn / reconcile path).
///
/// `recv` pulls from an internal mpsc; tests enqueue frames via `enqueue_output`.
/// When nothing is enqueued `recv` returns `None` immediately (the channel stays
/// open, so the pump stops cleanly without spinning).
pub struct FakeRuntime {
    state: Mutex<State>,
    fail_spawn: Mutex<bool>,
    tx: mpsc::UnboundedSender<SurfaceOutput>,
    // std::sync::Mutex because try_recv is sync.
    rx: Mutex<mpsc::UnboundedReceiver<SurfaceOutput>>,
}

impl Default for FakeRuntime {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            state: Mutex::default(),
            fail_spawn: Mutex::new(false),
            tx,
            rx: Mutex::new(rx),
        }
    }
}

impl FakeRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue a `SurfaceOutput` so a test can drive the pump without a daemon.
    pub fn enqueue_output(&self, surface: impl Into<String>, output: Output) {
        let _ = self.tx.send(SurfaceOutput {
            surface: surface.into(),
            output,
        });
    }

    /// Pull the next enqueued frame, or `None` if nothing is pending.
    pub async fn recv(&self) -> Option<SurfaceOutput> {
        self.rx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .try_recv()
            .ok()
    }

    /// Make the next `spawn` fail (to exercise the failed-effect path).
    pub fn fail_next_spawn(&self) {
        *self.fail_spawn.lock().unwrap() = true;
    }

    /// Seed a surface as already running in the daemon (to set up reconcile cases:
    /// running-but-no-row).
    pub fn seed_running(&self, surface: SurfaceId) {
        self.state.lock().unwrap().running.insert(surface);
    }

    pub fn is_running(&self, surface: &SurfaceId) -> bool {
        self.state.lock().unwrap().running.contains(surface)
    }

    pub fn calls(&self) -> Vec<RuntimeCall> {
        self.state.lock().unwrap().calls.clone()
    }

    fn record(&self, call: RuntimeCall) {
        self.state.lock().unwrap().calls.push(call);
    }

    pub async fn spawn(&self, request: SpawnRequest) -> Result<()> {
        self.record(RuntimeCall::Spawn(request.surface.clone()));
        if std::mem::replace(&mut *self.fail_spawn.lock().unwrap(), false) {
            return Err(Error::SurfaceRuntime {
                surface: request.surface.as_str().to_string(),
                reason: "fake spawn failure".to_string(),
            });
        }
        self.state.lock().unwrap().running.insert(request.surface);
        Ok(())
    }

    pub async fn stop(&self, surface: &SurfaceId) -> Result<()> {
        self.record(RuntimeCall::Stop(surface.clone()));
        self.state.lock().unwrap().running.remove(surface);
        Ok(())
    }

    pub async fn close(&self, surface: &SurfaceId) -> Result<()> {
        self.record(RuntimeCall::Close(surface.clone()));
        self.state.lock().unwrap().running.remove(surface);
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<SurfaceId>> {
        self.record(RuntimeCall::List);
        let mut ids: Vec<SurfaceId> = self.state.lock().unwrap().running.iter().cloned().collect();
        ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        Ok(ids)
    }

    pub async fn input(&self, surface: &SurfaceId, bytes: &[u8]) -> Result<()> {
        self.record(RuntimeCall::Input {
            surface: surface.clone(),
            bytes: bytes.to_vec(),
        });
        Ok(())
    }

    pub async fn resize(&self, surface: &SurfaceId, cols: u16, rows: u16) -> Result<()> {
        self.record(RuntimeCall::Resize {
            surface: surface.clone(),
            cols,
            rows,
        });
        Ok(())
    }

    pub async fn attach(&self, surface: &SurfaceId) -> Result<()> {
        self.record(RuntimeCall::Attach(surface.clone()));
        Ok(())
    }

    pub async fn detach(&self, surface: &SurfaceId) -> Result<()> {
        self.record(RuntimeCall::Detach(surface.clone()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::daemon_pty_api::{Geometry, SpawnRequest};

    fn request(surface: &str) -> SpawnRequest {
        SpawnRequest {
            surface: SurfaceId::from_string(surface),
            command: None,
            token: "t".into(),
            geometry: Geometry { cols: 80, rows: 24 },
            cwd: "/".into(),
        }
    }

    #[tokio::test]
    async fn spawn_marks_a_surface_running_and_list_reports_it() {
        let rt = FakeRuntime::new();
        rt.spawn(request("a")).await.unwrap();

        assert!(rt.is_running(&SurfaceId::from_string("a")));
        assert_eq!(rt.list().await.unwrap(), vec![SurfaceId::from_string("a")]);
    }

    #[tokio::test]
    async fn stop_removes_a_surface_from_the_running_set() {
        let rt = FakeRuntime::new();
        let id = SurfaceId::from_string("a");
        rt.spawn(request("a")).await.unwrap();
        rt.stop(&id).await.unwrap();

        assert!(!rt.is_running(&id));
        assert!(rt.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_seeded_surface_with_no_spawn_is_an_orphan_for_reconcile() {
        let rt = FakeRuntime::new();
        let id = SurfaceId::from_string("orphan");
        rt.seed_running(id.clone());

        assert_eq!(rt.list().await.unwrap(), vec![id]);
    }

    #[tokio::test]
    async fn fail_next_spawn_makes_the_spawn_error_and_leaves_it_not_running() {
        let rt = FakeRuntime::new();
        rt.fail_next_spawn();
        let result = rt.spawn(request("a")).await;

        assert!(matches!(result, Err(Error::SurfaceRuntime { .. })));
        assert!(!rt.is_running(&SurfaceId::from_string("a")));
    }

    #[tokio::test]
    async fn input_is_recorded_in_call_order() {
        let rt = FakeRuntime::new();
        let id = SurfaceId::from_string("a");
        rt.input(&id, b"ls\n").await.unwrap();

        assert_eq!(
            rt.calls(),
            vec![RuntimeCall::Input {
                surface: id,
                bytes: b"ls\n".to_vec(),
            }]
        );
    }
}
