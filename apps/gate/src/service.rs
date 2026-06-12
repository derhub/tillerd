//! Gate: service-host child. Gate owns teardown of subscriptions + registry (not service-host managed).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use service_host::host::{ServeContext, Service, ServiceConfig};
use tokio::net::UnixListener;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::agent_adapter::{AgentAdapter, V1Adapter};
use crate::endpoint::admin::Admin;
use crate::endpoint::dispatch::{dispatch, Faces};
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

/// The hosted tool name; the manifest derives from it.
const SERVICE_NAME: &str = "gate";

/// This binary's version, reported in the manifest.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Environment source of the admin token (distinct from any session token).
const ADMIN_TOKEN_ENV: &str = "TILLERD_GATE_ADMIN_TOKEN";

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
/// wired router, and the single socket's accept task (filled in `serve`).
pub struct Gate {
    version: String,
    base_override: Option<String>,
    registry: Arc<SessionRegistry>,
    subscriptions: Arc<Subscriptions>,
    admin: Arc<Admin>,
    router: Arc<Router>,
    tasks: Vec<JoinHandle<()>>,
}

impl Gate {
    /// Build the gate from the environment: the queue cap and the admin token (a
    /// random secret when unset).
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
            base_override: std::env::var(tillerd_paths::ENV_TILLERD_DIR).ok(),
            registry,
            subscriptions,
            admin,
            router,
            tasks: Vec::new(),
        }
    }

    /// Bind the single front-door socket at its deterministic path and run the
    /// accept loop: every connection is demultiplexed by its route preamble. The
    /// gate binds no TCP port and publishes no address file — the path derives from
    /// the runtime directory.
    fn bind_socket(&mut self, base: &Path) -> std::io::Result<()> {
        let path = tillerd_paths::gate_socket_in(base);
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path)?;
        let faces = Faces {
            registry: self.registry.clone(),
            admin: self.admin.clone(),
            subscriptions: self.subscriptions.clone(),
            router: self.router.clone(),
        };
        self.tasks.push(tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(dispatch(stream, faces.clone()));
            }
        }));
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
        let ServeContext {
            paths, ready, drain, ..
        } = ctx;
        self.bind_socket(paths.base_dir())?;
        // Listening: announce readiness so the host flips the manifest to `ready` for discovery.
        ready.signal();
        // The accept loop serves from its own task. Hold serve open until drained (return so the
        // host tears down) or until a stop signal cancels this future; either way `shutdown` aborts
        // the accept task.
        drain.draining().await;
        Ok(())
    }

    async fn shutdown(&mut self) {
        for task in self.tasks.drain(..) {
            task.abort();
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
    async fn binding_opens_one_socket_and_publishes_no_address_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut gate = gate();

        gate.bind_socket(dir.path()).unwrap();

        assert!(
            tokio::net::UnixStream::connect(tillerd_paths::gate_socket_in(dir.path()))
                .await
                .is_ok(),
            "the single front-door socket is bound"
        );
        for absent in [
            "gate.url",
            "gate-mcp.url",
            "gate-mcp.sock",
            "gate-hook.sock",
            "gate-tool.sock",
            "gate-subscribe.sock",
            "gate-admin.sock",
        ] {
            assert!(
                !dir.path().join(absent).exists(),
                "{absent} is gone: one socket, no per-face files, no published address"
            );
        }

        gate.shutdown().await;
    }

    #[tokio::test]
    async fn shutdown_stops_the_accept_loop() {
        let dir = tempfile::tempdir().unwrap();
        let mut gate = gate();
        gate.bind_socket(dir.path()).unwrap();

        gate.shutdown().await;

        assert!(
            gate.tasks.is_empty(),
            "the accept task is aborted, so the gate stops accepting new connections"
        );
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
}
