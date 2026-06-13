use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{OrchestratorError, Result};
use crate::launch::executor::{run as run_launch, LaunchItemResult, SurfaceLauncher};
use crate::launch::spec::{migrate, CommandRef, LaunchItem, LaunchSpec, CURRENT_SPEC_VERSION};
use crate::persistence::{NewSurface, SessionId, Store, SurfaceId, SurfaceKind};
use crate::surface::runtime::{ResolvedCommand, SurfaceEventSink, SurfaceRuntime};

/// Terminal dimensions a launched surface starts at; the renderer resizes on attach.
const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;

pub struct SurfaceApi {
    runtime: Arc<SurfaceRuntime>,
    store: Arc<dyn Store>,
}

/// Production `SurfaceLauncher`: brings a resolved launch item to life on the surface runtime
/// (the executor has already created the surface row and resolved the command).
struct RuntimeLauncher {
    runtime: Arc<SurfaceRuntime>,
}

impl SurfaceLauncher for RuntimeLauncher {
    async fn launch(
        &self,
        surface_id: &SurfaceId,
        kind: SurfaceKind,
        command: ResolvedCommand,
        cwd: Option<String>,
    ) -> Result<()> {
        let token = uuid::Uuid::new_v4().to_string();
        let work_dir = cwd.unwrap_or_else(default_cwd);
        self.runtime
            .launch_surface(
                surface_id.clone(),
                kind,
                Some(command),
                token,
                DEFAULT_COLS,
                DEFAULT_ROWS,
                work_dir,
            )
            .await
    }
}

impl SurfaceApi {
    pub fn new(store: Arc<dyn Store>, sink: Arc<dyn SurfaceEventSink>, socket: PathBuf) -> Self {
        let runtime = Arc::new(SurfaceRuntime::new(store.clone(), sink, socket));
        Self { runtime, store }
    }

    /// `correlation_id` (= the surface id) is bound into the span so the orchestrator's records for
    /// this operation join the daemon's on the same key across the process hop (design D5).
    #[tracing::instrument(skip_all, fields(correlation_id = %surface_id.as_str()))]
    pub async fn create_terminal_surface(
        &self,
        session_id: SessionId,
        surface_id: SurfaceId,
        placement: String,
        cols: u16,
        rows: u16,
        cwd: Option<String>,
    ) -> Result<SurfaceId> {
        tracing::info!(cols, rows, "creating terminal surface");
        let surface = self.store.create_surface(NewSurface {
            id: Some(surface_id.clone()),
            session_id,
            kind: SurfaceKind::Terminal,
            cwd: cwd.clone(),
            placement: Some(placement),
            worktree_id: None,
        })?;

        let token = uuid::Uuid::new_v4().to_string();
        let work_dir = cwd.unwrap_or_else(default_cwd);
        self.runtime
            .launch_surface(
                surface.id.clone(),
                SurfaceKind::Terminal,
                None,
                token,
                cols,
                rows,
                work_dir,
            )
            .await?;
        Ok(surface.id)
    }

    /// Instantiate a session's launch spec: run each item through the executor onto the runtime,
    /// best-effort (a failed item is recorded; the rest still run). A session without a stored spec
    /// launches nothing.
    pub async fn launch_session(&self, session_id: &SessionId) -> Result<Vec<LaunchItemResult>> {
        let session = self
            .store
            .get_session(session_id)?
            .ok_or_else(|| OrchestratorError::SessionNotFound(session_id.as_str().to_string()))?;
        let (version, blob) = match (session.spec_version, session.spec_json) {
            (Some(v), Some(b)) => (v, b),
            _ => return Ok(Vec::new()),
        };
        let (spec, _) = migrate(&blob, version)?;
        let launcher = RuntimeLauncher {
            runtime: self.runtime.clone(),
        };
        Ok(run_launch(&spec, session_id, &self.store, &launcher).await)
    }

    #[tracing::instrument(skip_all, fields(correlation_id = %surface.as_str()))]
    pub async fn input(&self, surface: &SurfaceId, bytes: &[u8]) -> Result<()> {
        tracing::debug!(bytes = bytes.len(), "surface input");
        self.runtime.input(surface, bytes).await
    }

    #[tracing::instrument(skip_all, fields(correlation_id = %surface.as_str()))]
    pub async fn resize(&self, surface: &SurfaceId, cols: u16, rows: u16) -> Result<()> {
        tracing::debug!(cols, rows, "surface resize");
        self.runtime.resize(surface, cols, rows).await
    }

    pub async fn resume_all(&self) -> Result<()> {
        self.runtime.resume_all().await
    }

    pub fn find_session_surface_by_placement(
        &self,
        session_id: &SessionId,
        placement: &str,
    ) -> Result<Option<SurfaceId>> {
        Ok(self
            .store
            .find_session_surface_by_placement(session_id, placement)?
            .map(|s| s.id))
    }

    /// Re-attach to an existing surface's live PTY, replaying scrollback. Errors if its session is
    /// gone, so the caller can create a fresh surface instead.
    pub async fn resume_surface(&self, surface: &SurfaceId) -> Result<()> {
        self.runtime.resume(surface.clone()).await
    }

    #[tracing::instrument(skip_all, fields(correlation_id = %surface.as_str()))]
    pub async fn detach(&self, surface: &SurfaceId) -> Result<()> {
        tracing::info!("detaching surface");
        self.runtime.detach(surface).await
    }

    pub async fn remove(&self, surface: &SurfaceId) -> Result<()> {
        self.store.soft_delete_surface(surface)?;
        self.runtime.remove(surface).await
    }

    // Returns a placement only; the renderer creates the surface at it (it owns the byte channel).
    pub fn spawn_surface(&self, session_id: &SessionId) -> Result<String> {
        let placement = uuid::Uuid::new_v4().to_string();
        let mut spec = self.session_spec(session_id)?;
        spec.items.push(LaunchItem {
            target: SurfaceKind::Terminal.as_str().to_string(),
            placement: Some(placement.clone()),
            command: CommandRef::Inline {
                executable: default_shell(),
                args: Vec::new(),
            },
            worktree: None,
        });
        spec.ensure_unique_placements()?;
        self.store_spec(session_id, &spec)?;
        Ok(placement)
    }

    pub async fn remove_surface(&self, session_id: &SessionId, surface: &SurfaceId) -> Result<()> {
        if let Some(placement) = self.store.get_surface(surface)?.and_then(|s| s.placement) {
            let mut spec = self.session_spec(session_id)?;
            spec.items
                .retain(|item| item.placement.as_deref() != Some(placement.as_str()));
            self.store_spec(session_id, &spec)?;
        }
        self.remove(surface).await
    }

    fn session_spec(&self, session_id: &SessionId) -> Result<LaunchSpec> {
        let session = self
            .store
            .get_session(session_id)?
            .ok_or_else(|| OrchestratorError::SessionNotFound(session_id.as_str().to_string()))?;
        match (session.spec_version, session.spec_json) {
            (Some(version), Some(blob)) => Ok(migrate(&blob, version)?.0),
            _ => Ok(LaunchSpec {
                version: CURRENT_SPEC_VERSION,
                items: Vec::new(),
            }),
        }
    }

    fn store_spec(&self, session_id: &SessionId, spec: &LaunchSpec) -> Result<()> {
        let blob = serde_json::to_string(spec)
            .map_err(|e| OrchestratorError::LaunchSpecInvalid(e.to_string()))?;
        self.store.set_session_spec(session_id, spec.version, &blob)
    }
}

fn default_cwd() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
}

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::memory::InMemoryStore;
    use crate::persistence::{NewLaunchTemplate, NewSession, ProjectId};
    use crate::surface::runtime::SurfaceEventSink;
    use daemon_pty_client::{encode_frame, FrameDecoder};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{UnixListener, UnixStream};

    struct NullSink;
    impl SurfaceEventSink for NullSink {
        fn on_bytes(&self, _surface: &SurfaceId, _bytes: &[u8]) {}
        fn on_status(&self, _surface: &SurfaceId, _status: &str) {}
        fn on_exit(&self, _surface: &SurfaceId, _qualifier: &str) {}
    }

    /// Answer one daemon connection: ack `hello` and every `spawn`.
    async fn serve_daemon_conn(stream: UnixStream) {
        let (mut rx, mut tx) = stream.into_split();
        let mut buf = vec![0u8; 1024];
        let mut dec = FrameDecoder::new();
        loop {
            let Ok(n) = rx.read(&mut buf).await else {
                return;
            };
            if n == 0 {
                return;
            }
            for frame in dec.push(&buf[..n]) {
                let meta: serde_json::Value =
                    serde_json::from_slice(&frame.meta).expect("meta json");
                match meta["type"].as_str() {
                    Some("hello") => {
                        let _ = tx
                            .write_all(&encode_frame(
                                br#"{"type":"hello-ack","version":1,"daemonVersion":"0","capabilities":["snapshot"]}"#,
                                None,
                            ))
                            .await;
                    }
                    Some("spawn") => {
                        let id = meta["sessionId"].as_str().unwrap_or("");
                        let ack = format!(r#"{{"type":"spawn-ack","sessionId":"{id}","pid":1}}"#);
                        let _ = tx.write_all(&encode_frame(ack.as_bytes(), None)).await;
                    }
                    _ => {}
                }
            }
        }
    }

    async fn fake_daemon(listener: UnixListener) {
        let (stream, _) = listener.accept().await.expect("accept");
        serve_daemon_conn(stream).await;
    }

    /// A daemon that serves every connection (one per launched surface).
    async fn fake_daemon_multi(listener: UnixListener) {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(serve_daemon_conn(stream));
        }
    }

    fn session_with_spec(store: &Arc<dyn Store>, spec_json: &str) -> SessionId {
        let template = store
            .create_launch_template(NewLaunchTemplate {
                project_id: ProjectId::unfiled(),
                spec_version: 1,
                spec_json: spec_json.to_string(),
            })
            .unwrap();
        store
            .create_session(NewSession {
                template_id: Some(template.id),
                ..Default::default()
            })
            .unwrap()
            .id
    }

    #[tokio::test]
    async fn launch_session_runs_each_spec_item_onto_the_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let daemon = tokio::spawn(fake_daemon_multi(listener));

        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let session_id = session_with_spec(
            &store,
            r#"{"version":1,"items":[
                {"target":"terminal","command":{"executable":"/bin/sh","args":[]}},
                {"target":"terminal","command":{"executable":"/bin/sh","args":[]}}
            ]}"#,
        );
        let api = SurfaceApi::new(store.clone(), Arc::new(NullSink), sock);

        let results = api
            .launch_session(&session_id)
            .await
            .expect("launch_session");

        assert_eq!(results.len(), 2);
        assert!(
            results.iter().all(|r| r.error.is_none()),
            "every item launches: {results:?}"
        );
        assert_eq!(api.runtime.proxy_count().await, 2);
        daemon.abort();
    }

    #[tokio::test]
    async fn launch_session_without_a_spec_launches_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let session = store.create_session(NewSession::default()).unwrap();
        let api = SurfaceApi::new(store, Arc::new(NullSink), sock);

        let results = api
            .launch_session(&session.id)
            .await
            .expect("launch_session");

        assert!(results.is_empty());
        assert_eq!(api.runtime.proxy_count().await, 0);
    }

    #[tokio::test]
    async fn create_terminal_surface_persists_a_row_and_starts_one_proxy() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let daemon = tokio::spawn(fake_daemon(listener));

        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let session = store
            .create_session(crate::persistence::NewSession::default())
            .unwrap();
        let api = SurfaceApi::new(store.clone(), Arc::new(NullSink), sock);

        let id = SurfaceId::from_string("surf-api-1");
        let returned = api
            .create_terminal_surface(
                session.id,
                id.clone(),
                "p".into(),
                80,
                24,
                Some("/tmp".into()),
            )
            .await
            .expect("create");

        assert_eq!(returned, id);
        let row = store.get_surface(&id).expect("get").expect("row exists");
        assert_eq!(row.kind, SurfaceKind::Terminal);
        assert_eq!(row.cwd.as_deref(), Some("/tmp"));
        assert_eq!(row.placement.as_deref(), Some("p"));
        assert_eq!(api.runtime.proxy_count().await, 1);
        daemon.abort();
    }

    #[test]
    fn find_session_surface_by_placement_resolves_to_its_own_surface() {
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let with_surface = store.create_session(NewSession::default()).unwrap();
        let without_surface = store.create_session(NewSession::default()).unwrap();
        let surface = store
            .create_surface(NewSurface {
                id: Some(SurfaceId::from_string("revisit-surf")),
                session_id: with_surface.id.clone(),
                kind: SurfaceKind::Terminal,
                cwd: None,
                placement: Some("main".into()),
                worktree_id: None,
            })
            .unwrap();
        let api = SurfaceApi::new(store, Arc::new(NullSink), "/tmp/unused.sock".into());

        // The revisit path resolves (session, placement) to its own surface, and to None otherwise.
        assert_eq!(
            api.find_session_surface_by_placement(&with_surface.id, "main")
                .unwrap(),
            Some(surface.id),
        );
        assert_eq!(
            api.find_session_surface_by_placement(&without_surface.id, "main")
                .unwrap(),
            None,
        );
    }

    #[test]
    fn spawn_surface_appends_a_launch_item_at_a_minted_placement() {
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let session = store.create_session(NewSession::default()).unwrap();
        let api = SurfaceApi::new(store.clone(), Arc::new(NullSink), "/tmp/unused.sock".into());

        let placement = api.spawn_surface(&session.id).unwrap();
        let stored = store.get_session(&session.id).unwrap().unwrap();
        let spec = crate::launch::spec::parse_spec(&stored.spec_json.unwrap()).unwrap();
        assert_eq!(spec.items.len(), 1);
        assert_eq!(spec.items[0].placement.as_deref(), Some(placement.as_str()));
    }

    #[tokio::test]
    async fn remove_surface_drops_its_launch_item() {
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let session = store.create_session(NewSession::default()).unwrap();
        let api = SurfaceApi::new(store.clone(), Arc::new(NullSink), "/tmp/unused.sock".into());

        let placement = api.spawn_surface(&session.id).unwrap();
        let surface = store
            .create_surface(NewSurface {
                id: Some(SurfaceId::from_string("close-surf")),
                session_id: session.id.clone(),
                kind: SurfaceKind::Terminal,
                cwd: None,
                placement: Some(placement),
                worktree_id: None,
            })
            .unwrap();

        api.remove_surface(&session.id, &surface.id).await.unwrap();

        let stored = store.get_session(&session.id).unwrap().unwrap();
        let spec = crate::launch::spec::parse_spec(&stored.spec_json.unwrap()).unwrap();
        assert!(spec.items.is_empty());
        assert!(store.get_surface(&surface.id).unwrap().is_none());
    }
}
