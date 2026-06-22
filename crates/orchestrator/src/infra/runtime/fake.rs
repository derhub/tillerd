//! In-memory [`SurfaceRuntime`] for tests: no socket, no daemon. Tracks which
//! surfaces it considers running and records the calls made against it so app-layer
//! tests can assert the side-effect shape (persist intent -> effect -> record ->
//! reconcile) without a live daemon.

use std::collections::HashSet;
use std::sync::Mutex;

use super::{BoxFut, SpawnRequest, SurfaceRuntime};
use crate::entities::SurfaceId;
use crate::shared::Error;

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
#[derive(Default)]
pub struct FakeRuntime {
    state: Mutex<State>,
    fail_spawn: Mutex<bool>,
}

impl FakeRuntime {
    pub fn new() -> Self {
        Self::default()
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
}

impl SurfaceRuntime for FakeRuntime {
    fn spawn<'a>(&'a self, request: SpawnRequest) -> BoxFut<'a, ()> {
        Box::pin(async move {
            self.record(RuntimeCall::Spawn(request.surface.clone()));
            if std::mem::replace(&mut *self.fail_spawn.lock().unwrap(), false) {
                return Err(Error::SurfaceRuntime {
                    surface: request.surface.as_str().to_string(),
                    reason: "fake spawn failure".to_string(),
                });
            }
            self.state.lock().unwrap().running.insert(request.surface);
            Ok(())
        })
    }

    fn stop<'a>(&'a self, surface: &'a SurfaceId) -> BoxFut<'a, ()> {
        Box::pin(async move {
            self.record(RuntimeCall::Stop(surface.clone()));
            self.state.lock().unwrap().running.remove(surface);
            Ok(())
        })
    }

    fn close<'a>(&'a self, surface: &'a SurfaceId) -> BoxFut<'a, ()> {
        Box::pin(async move {
            self.record(RuntimeCall::Close(surface.clone()));
            self.state.lock().unwrap().running.remove(surface);
            Ok(())
        })
    }

    fn list<'a>(&'a self) -> BoxFut<'a, Vec<SurfaceId>> {
        Box::pin(async move {
            self.record(RuntimeCall::List);
            let mut ids: Vec<SurfaceId> =
                self.state.lock().unwrap().running.iter().cloned().collect();
            ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            Ok(ids)
        })
    }

    fn input<'a>(&'a self, surface: &'a SurfaceId, bytes: &'a [u8]) -> BoxFut<'a, ()> {
        Box::pin(async move {
            self.record(RuntimeCall::Input {
                surface: surface.clone(),
                bytes: bytes.to_vec(),
            });
            Ok(())
        })
    }

    fn resize<'a>(&'a self, surface: &'a SurfaceId, cols: u16, rows: u16) -> BoxFut<'a, ()> {
        Box::pin(async move {
            self.record(RuntimeCall::Resize {
                surface: surface.clone(),
                cols,
                rows,
            });
            Ok(())
        })
    }

    fn attach<'a>(&'a self, surface: &'a SurfaceId) -> BoxFut<'a, ()> {
        Box::pin(async move {
            self.record(RuntimeCall::Attach(surface.clone()));
            Ok(())
        })
    }

    fn detach<'a>(&'a self, surface: &'a SurfaceId) -> BoxFut<'a, ()> {
        Box::pin(async move {
            self.record(RuntimeCall::Detach(surface.clone()));
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::SurfaceKind;
    use crate::infra::runtime::{Geometry, SpawnRequest};

    fn request(surface: &str) -> SpawnRequest {
        SpawnRequest {
            surface: SurfaceId::from_string(surface),
            kind: SurfaceKind::Terminal,
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
