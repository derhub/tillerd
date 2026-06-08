//! The gate as a hosted `service-host` Service.
//!
//! `service-host` owns path resolution, the manifest, signal handling, and the
//! unauthenticated liveness probe (the gate's only health face). The gate supplies
//! its identity and its serve behavior: it binds the five loopback faces (hook,
//! tool, subscribe, admin, mcp) and tracks their tasks, then tears down the live
//! subscriptions and session registry on shutdown — neither is a `service-host`
//! child, so the gate owns their teardown.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use service_host::host::{ServeContext, Service, ServiceConfig};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::agent_adapter::{AgentAdapter, V1Adapter};
use crate::endpoint::admin::Admin;
use crate::endpoint::{admin, hook, mcp, subscribe, tool};
use crate::middleware::auth::Auth;
use crate::middleware::fanout::FanOut;
use crate::middleware::normalize::Normalize;
use crate::middleware::observe::{ObservationRecord, Observe, ObserveSink, RecordOutcome};
use crate::middleware::passthrough::PassThrough;
use crate::middleware::{seq, Middleware};
use crate::registry::SessionRegistry;
use crate::router::Router;
use crate::subscription::Subscriptions;
use crate::{Kind, Token};

/// The hosted tool name; the manifest and probe socket derive from it.
const SERVICE_NAME: &str = "gate";

/// This binary's version, reported in the manifest and by the probe.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Environment source of the admin token (distinct from any session token).
const ADMIN_TOKEN_ENV: &str = "ATHING_GATE_ADMIN_TOKEN";

/// The production observation sink: records flow through `tracing`.
struct LogSink;

impl ObserveSink for LogSink {
    fn emit(&self, record: ObservationRecord) {
        let outcome = match &record.outcome {
            RecordOutcome::Accepted => "accepted",
            RecordOutcome::Rejected(reason) => reason.as_str(),
        };
        tracing::info!(
            ts = record.ts,
            session_id = %record.session_id.0,
            correlation_id = %record.correlation_id.0,
            component = record.component,
            kind = ?record.kind,
            event_type = ?record.event_type,
            outcome,
            latency_ms = record.latency_ms,
            fanout_n = ?record.fanout_n,
            dropped_n = ?record.dropped_n,
            "gate observation"
        );
    }
}

/// The gate service: the shared registry and subscriptions, the admin face, the
/// wired router, and the loopback face tasks (filled in `serve`).
pub struct Gate {
    version: String,
    base_override: Option<String>,
    port: u16,
    max_body: usize,
    mcp_transport: mcp::Transport,
    mcp_port: u16,
    mcp_sidecar: Option<PathBuf>,
    registry: Arc<SessionRegistry>,
    subscriptions: Arc<Subscriptions>,
    admin: Arc<Admin>,
    router: Arc<Router>,
    tasks: Vec<JoinHandle<()>>,
}

impl Gate {
    /// Build the gate from the environment: the queue cap, the hook port and body
    /// cap, and the admin token (a random secret when unset).
    pub fn from_env() -> Self {
        let registry = Arc::new(SessionRegistry::new());
        let subscriptions = Arc::new(Subscriptions::from_env());
        let adapter: Arc<dyn AgentAdapter> = Arc::new(V1Adapter);
        let admin_token =
            std::env::var(ADMIN_TOKEN_ENV).unwrap_or_else(|_| Uuid::new_v4().to_string());
        let admin = Arc::new(Admin::new(&Token::new(admin_token), registry.clone()));
        let router = build_router(
            registry.clone(),
            subscriptions.clone(),
            adapter,
            Arc::new(LogSink),
        );
        Self {
            version: VERSION.to_string(),
            base_override: std::env::var("ATHING_DIR").ok(),
            port: hook::port_from_env(),
            max_body: hook::max_body_from_env(),
            mcp_transport: mcp::transport_from_env(),
            mcp_port: mcp::port_from_env(),
            mcp_sidecar: None,
            registry,
            subscriptions,
            admin,
            router,
            tasks: Vec::new(),
        }
    }

    /// Bind every loopback face, publish the hook URL, and track each face's task.
    async fn bind_faces(&mut self, base: &Path) -> std::io::Result<()> {
        let listener = hook::bind(self.port).await?;
        hook::write_gate_url(&base.join("gate.url"), listener.local_addr()?)?;
        let app = hook::app(self.router.clone(), self.max_body);
        self.tasks.push(tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        }));
        self.tasks.push(tool::serve(
            base.join("gate-tool.sock"),
            self.router.clone(),
        )?);
        self.tasks.push(subscribe::serve(
            base.join("gate-subscribe.sock"),
            self.subscriptions.clone(),
        )?);
        self.tasks.push(admin::serve(
            base.join("gate-admin.sock"),
            self.admin.clone(),
        )?);
        self.bind_mcp_face(base).await?;
        Ok(())
    }

    /// Bind the configured MCP transport, publish its sidecar, and track its task.
    async fn bind_mcp_face(&mut self, base: &Path) -> std::io::Result<()> {
        match self.mcp_transport {
            mcp::Transport::Http => {
                let listener = mcp::bind(self.mcp_port).await?;
                let sidecar = base.join("gate-mcp.url");
                mcp::write_mcp_url(&sidecar, listener.local_addr()?)?;
                let app = mcp::http_app(self.router.clone(), self.registry.clone());
                self.tasks.push(tokio::spawn(async move {
                    let _ = axum::serve(listener, app).await;
                }));
                self.mcp_sidecar = Some(sidecar);
            }
            mcp::Transport::Socket => {
                let sidecar = base.join("gate-mcp.sock");
                self.tasks.push(mcp::serve_socket(
                    sidecar.clone(),
                    self.router.clone(),
                    self.registry.clone(),
                )?);
                self.mcp_sidecar = Some(sidecar);
            }
        }
        Ok(())
    }
}

fn build_router(
    registry: Arc<SessionRegistry>,
    subscriptions: Arc<Subscriptions>,
    adapter: Arc<dyn AgentAdapter>,
    sink: Arc<dyn ObserveSink>,
) -> Arc<Router> {
    let globals = vec![
        Arc::new(Observe::new(sink)) as Arc<dyn Middleware>,
        Arc::new(Auth::new(registry)),
    ];
    let hook_route = seq(vec![
        Arc::new(Normalize::new(adapter)) as Arc<dyn Middleware>,
        Arc::new(FanOut::new(subscriptions)),
    ]);
    let routes = HashMap::from([
        (Kind::Hook, hook_route),
        (Kind::ToolCall, Arc::new(PassThrough) as Arc<dyn Middleware>),
        (
            Kind::ToolResult,
            Arc::new(PassThrough) as Arc<dyn Middleware>,
        ),
        (Kind::Mcp, Arc::new(PassThrough) as Arc<dyn Middleware>),
    ]);
    Arc::new(Router::new(globals, routes))
}

impl Service for Gate {
    fn config(&self) -> ServiceConfig {
        ServiceConfig::new(SERVICE_NAME, self.version.clone())
            .with_base_override(self.base_override.clone())
    }

    async fn serve(&mut self, ctx: ServeContext) -> std::io::Result<()> {
        let ServeContext { paths, .. } = ctx;
        self.bind_faces(paths.base_dir()).await?;
        // The faces serve from their own tasks; hold serve open until the host's
        // stop signal cancels it, then `shutdown` aborts the tasks.
        std::future::pending::<std::io::Result<()>>().await
    }

    async fn shutdown(&mut self) {
        for task in self.tasks.drain(..) {
            task.abort();
        }
        if let Some(sidecar) = self.mcp_sidecar.take() {
            let _ = std::fs::remove_file(sidecar);
        }
        self.subscriptions.clear();
        self.registry.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::SessionId;
    use tokio::sync::broadcast::error::RecvError;

    fn gate() -> Gate {
        let registry = Arc::new(SessionRegistry::new());
        let subscriptions = Arc::new(Subscriptions::with_capacity(8));
        let adapter: Arc<dyn AgentAdapter> = Arc::new(V1Adapter);
        let admin = Arc::new(Admin::new(&Token::new("admin"), registry.clone()));
        let router = build_router(
            registry.clone(),
            subscriptions.clone(),
            adapter,
            Arc::new(LogSink),
        );
        Gate {
            version: "9.9.9".into(),
            base_override: None,
            port: 0,
            max_body: 1 << 20,
            mcp_transport: mcp::Transport::Http,
            mcp_port: 0,
            mcp_sidecar: None,
            registry,
            subscriptions,
            admin,
            router,
            tasks: Vec::new(),
        }
    }

    #[test]
    fn config_identifies_the_tool_as_gate() {
        let config = gate().config();

        assert_eq!(config.name, "gate");
        assert_eq!(config.version, "9.9.9");
    }

    #[tokio::test]
    async fn binding_publishes_the_hook_url_and_opens_every_ipc_face() {
        let dir = tempfile::tempdir().unwrap();
        let mut gate = gate();

        gate.bind_faces(dir.path()).await.unwrap();

        let url = std::fs::read_to_string(dir.path().join("gate.url")).unwrap();
        assert!(
            url.starts_with("http://127.0.0.1:"),
            "the hook url is published"
        );
        let mcp_url = std::fs::read_to_string(dir.path().join("gate-mcp.url")).unwrap();
        assert!(
            mcp_url.starts_with("http://127.0.0.1:") && mcp_url.ends_with("/mcp"),
            "the mcp endpoint is published beside the hook url"
        );
        for socket in ["gate-tool.sock", "gate-subscribe.sock", "gate-admin.sock"] {
            assert!(
                tokio::net::UnixStream::connect(dir.path().join(socket))
                    .await
                    .is_ok(),
                "the {socket} face is bound"
            );
        }

        gate.shutdown().await;
    }

    #[tokio::test]
    async fn shutdown_clears_the_session_registry() {
        let mut gate = gate();
        let session = SessionId("s".into());
        gate.registry.register(session.clone(), &Token::new("t"));

        gate.shutdown().await;

        assert!(gate.registry.verify(&session, &Token::new("t")).is_none());
    }

    #[tokio::test]
    async fn shutdown_closes_active_subscriptions() {
        let mut gate = gate();
        let mut rx = gate.subscriptions.subscribe(&SessionId("s".into()));

        gate.shutdown().await;

        assert!(matches!(rx.recv().await, Err(RecvError::Closed)));
    }

    #[derive(Default)]
    struct FakeSink {
        records: std::sync::Mutex<Vec<ObservationRecord>>,
    }

    impl ObserveSink for FakeSink {
        fn emit(&self, record: ObservationRecord) {
            self.records.lock().unwrap().push(record);
        }
    }

    fn mcp_router(sink: Arc<dyn ObserveSink>) -> Arc<Router> {
        let registry = Arc::new(SessionRegistry::new());
        registry.register(SessionId("s".into()), &Token::new("secret"));
        build_router(
            registry,
            Arc::new(Subscriptions::with_capacity(8)),
            Arc::new(V1Adapter),
            sink,
        )
    }

    fn mcp_inbound() -> crate::router::Inbound {
        crate::router::Inbound {
            kind: Kind::Mcp,
            session: SessionId("s".into()),
            correlation: None,
            token: Token::new("secret"),
            body: bytes::Bytes::from_static(b"{}"),
        }
    }

    #[tokio::test]
    async fn an_authenticated_mcp_inbound_flows_through_the_global_onion_to_a_terminal_outcome() {
        let router = mcp_router(Arc::new(FakeSink::default()));

        let out = router.handle(mcp_inbound()).await.unwrap();

        assert_eq!(
            out,
            crate::Outbound::Forward(bytes::Bytes::from_static(b"{}")),
            "the mcp route forwards through observe and auth"
        );
    }

    #[tokio::test]
    async fn a_routed_mcp_inbound_is_observed_exactly_once_with_a_correlation_id() {
        let sink = Arc::new(FakeSink::default());
        let router = mcp_router(sink.clone());

        router.handle(mcp_inbound()).await.unwrap();

        let records = sink.records.lock().unwrap();
        assert_eq!(records.len(), 1, "exactly one observation per mcp inbound");
        assert_eq!(records[0].kind, Kind::Mcp);
        assert!(
            !records[0].correlation_id.0.is_empty(),
            "the observation carries a correlation id"
        );
    }

    #[tokio::test]
    async fn shutdown_stops_the_mcp_face_and_removes_its_discovery_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let mut gate = gate();
        gate.bind_faces(dir.path()).await.unwrap();
        let sidecar = dir.path().join("gate-mcp.url");
        assert!(
            sidecar.exists(),
            "the mcp endpoint is published after binding"
        );

        gate.shutdown().await;

        assert!(
            gate.tasks.is_empty(),
            "every face task is aborted, so the face stops accepting new connections"
        );
        assert!(
            !sidecar.exists(),
            "the discovery sidecar is removed on clean shutdown"
        );
    }

    #[tokio::test]
    async fn the_configured_transport_is_the_only_one_bound() {
        let dir = tempfile::tempdir().unwrap();
        let mut gate = gate();
        gate.mcp_transport = mcp::Transport::Socket;

        gate.bind_faces(dir.path()).await.unwrap();

        assert!(
            dir.path().join("gate-mcp.sock").exists(),
            "the socket transport binds its socket"
        );
        assert!(
            !dir.path().join("gate-mcp.url").exists(),
            "no http endpoint is published when the socket transport is selected"
        );

        gate.shutdown().await;
    }
}
