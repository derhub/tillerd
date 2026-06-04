use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use rmcp::service::{Peer, RunningServiceCancellationToken};
use rmcp::RoleClient;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::backend;
use crate::config::{BackendSpec, McpConfig};
use crate::front::FrontPeer;
use crate::registry::Registry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendState {
    Disabled,
    Idle,
    Starting,
    Ready,
    Unhealthy,
    Restarting,
    Failed,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct ReloadReport {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub restarted: Vec<String>,
    pub updated: Vec<String>,
    pub unchanged: Vec<String>,
    pub failed: Vec<String>,
}

fn spawn_fields_differ(a: &BackendSpec, b: &BackendSpec) -> bool {
    a.command != b.command
        || a.args != b.args
        || a.env != b.env
        || a.url != b.url
        || a.headers != b.headers
}

fn policy_differs(a: &BackendSpec, b: &BackendSpec) -> bool {
    a.allowed_tools != b.allowed_tools || a.lazy != b.lazy
}

#[derive(Debug, Clone)]
pub struct Tuning {
    pub restart_budget: u32,
    pub backoff_base: Duration,
    pub backoff_ceiling: Duration,
    pub idle_timeout: Duration,
    pub liveness_interval: Duration,
    pub liveness_timeout: Duration,
    pub drain_timeout: Duration,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            restart_budget: 8,
            backoff_base: Duration::from_millis(200),
            backoff_ceiling: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300),
            liveness_interval: Duration::from_secs(15),
            liveness_timeout: Duration::from_secs(5),
            drain_timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Clone)]
pub struct Supervisor {
    config: Arc<RwLock<McpConfig>>,
    registry: Registry,
    front: FrontPeer,
    refresh_tx: mpsc::UnboundedSender<String>,
    peers: Arc<RwLock<HashMap<String, Peer<RoleClient>>>>,
    cancels: Arc<RwLock<HashMap<String, RunningServiceCancellationToken>>>,
    states: Arc<RwLock<HashMap<String, BackendState>>>,
    spawn_lock: Arc<Mutex<()>>,
    inflight: Arc<RwLock<HashMap<String, Arc<std::sync::atomic::AtomicU64>>>>,
    draining: Arc<RwLock<std::collections::HashSet<String>>>,
    drain_gate: Arc<tokio::sync::Notify>,
    tuning: Arc<Tuning>,
}

pub struct CallGuard {
    count: Arc<std::sync::atomic::AtomicU64>,
    gate: Arc<tokio::sync::Notify>,
}

impl Drop for CallGuard {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::Release);
        self.gate.notify_waiters();
    }
}

impl Supervisor {
    pub fn new(
        config: McpConfig,
        registry: Registry,
        front: FrontPeer,
    ) -> (Self, mpsc::UnboundedReceiver<String>) {
        Self::new_tuned(config, registry, front, Tuning::default())
    }

    pub fn new_tuned(
        config: McpConfig,
        registry: Registry,
        front: FrontPeer,
        tuning: Tuning,
    ) -> (Self, mpsc::UnboundedReceiver<String>) {
        let (refresh_tx, refresh_rx) = mpsc::unbounded_channel();
        let s = Self {
            config: Arc::new(RwLock::new(config)),
            registry,
            front,
            refresh_tx,
            peers: Default::default(),
            cancels: Default::default(),
            states: Default::default(),
            spawn_lock: Default::default(),
            inflight: Default::default(),
            draining: Default::default(),
            drain_gate: Default::default(),
            tuning: Arc::new(tuning),
        };
        (s, refresh_rx)
    }

    pub async fn enter_call(&self, name: &str) -> CallGuard {
        loop {
            if self.draining.read().await.contains(name) {
                self.drain_gate.notified().await;
                continue;
            }
            let count = self
                .inflight
                .write()
                .await
                .entry(name.to_string())
                .or_default()
                .clone();
            count.fetch_add(1, Ordering::Release);
            return CallGuard {
                count,
                gate: self.drain_gate.clone(),
            };
        }
    }

    async fn begin_drain(&self, name: &str) {
        self.draining.write().await.insert(name.to_string());
        let count = self.inflight.read().await.get(name).cloned();
        if let Some(c) = count {
            let deadline = tokio::time::Instant::now() + self.tuning.drain_timeout;
            // Force-proceed once the deadline passes even if calls remain.
            while c.load(Ordering::Acquire) > 0 && tokio::time::Instant::now() < deadline {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }

    async fn end_drain(&self, name: &str) {
        self.draining.write().await.remove(name);
        self.drain_gate.notify_waiters();
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    async fn set_state(&self, name: &str, state: BackendState) {
        self.states.write().await.insert(name.to_string(), state);
    }

    pub async fn state(&self, name: &str) -> Option<BackendState> {
        self.states.read().await.get(name).copied()
    }

    pub async fn states(&self) -> HashMap<String, BackendState> {
        self.states.read().await.clone()
    }

    fn allowed(spec: &BackendSpec) -> Option<Vec<String>> {
        spec.allowed_tools.clone()
    }

    pub async fn start(&self) {
        let servers: Vec<(String, BackendSpec)> = self
            .config
            .read()
            .await
            .servers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (name, spec) in servers.iter() {
            if spec.lazy {
                self.set_state(name, BackendState::Idle).await;
                if let Err(e) = self.boot_index_release(name, spec).await {
                    tracing::warn!(%name, error=%e, "lazy boot-index failed");
                }
            } else {
                self.spawn_supervise(name.clone(), spec.clone());
            }
        }
    }

    // Index a lazy backend once, then drop the connection; its tools stay
    // indexed so listings cover it without keeping the process warm.
    async fn boot_index_release(&self, name: &str, spec: &BackendSpec) -> anyhow::Result<()> {
        let running = backend::connect(name, spec, self.front.clone(), self.refresh_tx.clone())
            .await?;
        let client = running.peer().clone();
        backend::index(name, &client, &self.registry, Self::allowed(spec).as_deref()).await?;
        let _ = running.cancel().await;
        Ok(())
    }

    // A loop (not recursion) so the future stays Send across tokio::spawn.
    fn spawn_supervise(&self, name: String, spec: BackendSpec) {
        let this = self.clone();
        tokio::spawn(async move {
            let mut attempt: u32 = 0;
            loop {
                this.set_state(
                    &name,
                    if attempt == 0 {
                        BackendState::Starting
                    } else {
                        BackendState::Restarting
                    },
                )
                .await;

                match backend::connect(&name, &spec, this.front.clone(), this.refresh_tx.clone())
                    .await
                {
                    Ok(running) => {
                        let client = running.peer().clone();
                        if let Err(e) = backend::index(
                            &name,
                            &client,
                            &this.registry,
                            Self::allowed(&spec).as_deref(),
                        )
                        .await
                        {
                            tracing::warn!(%name, error=%e, "index failed");
                        }
                        this.cancels
                            .write()
                            .await
                            .insert(name.clone(), running.cancellation_token());
                        this.peers.write().await.insert(name.clone(), client.clone());
                        this.set_state(&name, BackendState::Ready).await;
                        this.notify_front_tools_changed().await;
                        tracing::info!(%name, "backend ready");
                        attempt = 0;

                        // Probe alongside the exit watch to catch a wedged-but-alive process.
                        let probe_client = client.clone();
                        let interval = this.tuning.liveness_interval;
                        let probe_timeout = this.tuning.liveness_timeout;
                        let probe = async move {
                            loop {
                                tokio::time::sleep(interval).await;
                                let ok = matches!(
                                    tokio::time::timeout(
                                        probe_timeout,
                                        probe_client.list_tools(None),
                                    )
                                    .await,
                                    Ok(Ok(_))
                                );
                                if !ok {
                                    break;
                                }
                            }
                        };
                        tokio::select! {
                            reason = running.waiting() => {
                                tracing::warn!(%name, ?reason, "backend exited");
                            }
                            _ = probe => {
                                tracing::warn!(%name, "backend failed liveness probe; restarting");
                            }
                        }
                        this.peers.write().await.remove(&name);
                        this.cancels.write().await.remove(&name);
                        this.registry.drop_backend(&name);

                        // A deliberate stop/idle-release set Disabled: end the loop.
                        if matches!(this.state(&name).await, Some(BackendState::Disabled)) {
                            this.notify_front_tools_changed().await;
                            return;
                        }
                        this.set_state(&name, BackendState::Unhealthy).await;
                        this.notify_front_tools_changed().await;
                    }
                    Err(e) => {
                        tracing::warn!(%name, error=%e, "connect failed");
                    }
                }

                attempt += 1;
                if attempt > this.tuning.restart_budget {
                    tracing::error!(%name, "restart budget exhausted; Failed");
                    this.set_state(&name, BackendState::Failed).await;
                    return;
                }
                let base = this.tuning.backoff_base;
                let backoff = (base * 2u32.pow((attempt - 1).min(8))).min(this.tuning.backoff_ceiling);
                tokio::time::sleep(backoff).await;
            }
        });
    }

    pub async fn peer(&self, name: &str) -> Option<Peer<RoleClient>> {
        if let Some(p) = self.peers.read().await.get(name).cloned() {
            return Some(p);
        }
        let spec = self.config.read().await.servers.get(name).cloned()?;
        let _guard = self.spawn_lock.lock().await;
        if let Some(p) = self.peers.read().await.get(name).cloned() {
            return Some(p);
        }

        // Don't start a second loop if one is already supervising this backend.
        let already_supervised = matches!(
            self.state(name).await,
            Some(BackendState::Starting | BackendState::Ready | BackendState::Restarting
                | BackendState::Unhealthy)
        );
        if !already_supervised {
            self.spawn_supervise(name.to_string(), spec.clone());
        }

        // Cold-start grace: wait for the handshake before the caller's call runs.
        let deadline = self.tuning.idle_timeout.min(Duration::from_secs(30));
        let start = tokio::time::Instant::now();
        loop {
            if let Some(p) = self.peers.read().await.get(name).cloned() {
                if spec.lazy {
                    self.arm_idle_shutdown(name.to_string());
                }
                return Some(p);
            }
            if matches!(self.state(name).await, Some(BackendState::Failed))
                || start.elapsed() >= deadline
            {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    fn arm_idle_shutdown(&self, name: String) {
        let this = self.clone();
        let idle = self.tuning.idle_timeout;
        tokio::spawn(async move {
            tokio::time::sleep(idle).await;
            if this.peers.write().await.remove(&name).is_some() {
                this.set_state(&name, BackendState::Disabled).await;
                if let Some(token) = this.cancels.write().await.remove(&name) {
                    token.cancel();
                }
                tracing::info!(%name, "lazy backend idle-released");
            }
        });
    }

    async fn notify_front_tools_changed(&self) {
        if let Some(peer) = self.front.get() {
            let _ = peer.notify_tool_list_changed().await;
        }
    }

    async fn reindex(&self, name: &str) {
        let Some(spec) = self.config.read().await.servers.get(name).cloned() else {
            return;
        };
        let Some(client) = self.peers.read().await.get(name).cloned() else {
            return;
        };
        if let Err(e) =
            backend::index(name, &client, &self.registry, Self::allowed(&spec).as_deref()).await
        {
            tracing::warn!(%name, error=%e, "reindex failed");
            return;
        }
        self.notify_front_tools_changed().await;
    }

    pub fn run_refresh_loop(self: &Arc<Self>, mut rx: mpsc::UnboundedReceiver<String>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(name) = rx.recv().await {
                this.reindex(&name).await;
            }
        });
    }

    pub async fn stop(&self, name: &str) {
        self.set_state(name, BackendState::Disabled).await;
        self.peers.write().await.remove(name);
        if let Some(token) = self.cancels.write().await.remove(name) {
            token.cancel();
        }
        self.registry.drop_backend(name);
        self.notify_front_tools_changed().await;
    }

    pub async fn start_one(&self, name: &str) {
        if let Some(spec) = self.config.read().await.servers.get(name).cloned() {
            self.spawn_supervise(name.to_string(), spec);
        }
    }

    pub async fn restart(&self, name: &str) {
        self.begin_drain(name).await;
        self.stop(name).await;
        self.start_one(name).await;
        self.end_drain(name).await;
    }

    pub async fn configured_names(&self) -> Vec<String> {
        self.config.read().await.servers.keys().cloned().collect()
    }

    pub async fn reload(&self, new: McpConfig) -> ReloadReport {
        let mut report = ReloadReport::default();
        let old = self.config.read().await.clone();
        // Swap first so respawns see the new specs.
        *self.config.write().await = new.clone();

        let old_names: std::collections::HashSet<&String> = old.servers.keys().collect();
        let new_names: std::collections::HashSet<&String> = new.servers.keys().collect();

        for name in old_names.difference(&new_names) {
            self.stop(name).await;
            report.removed.push((*name).clone());
        }
        for name in new_names.difference(&old_names) {
            self.start_one(name).await;
            report.added.push((*name).clone());
        }
        for name in new_names.intersection(&old_names) {
            let o = &old.servers[*name];
            let n = &new.servers[*name];
            if spawn_fields_differ(o, n) {
                self.restart(name).await;
                report.restarted.push((*name).clone());
            } else if policy_differs(o, n) {
                self.reindex(name).await;
                report.updated.push((*name).clone());
            } else {
                report.unchanged.push((*name).clone());
            }
        }
        report
    }

    pub async fn shutdown(&self) {
        let names: Vec<String> = self.states().await.into_keys().collect();
        for name in names {
            self.set_state(&name, BackendState::Disabled).await;
        }
        self.peers.write().await.clear();
        let tokens: Vec<_> = self.cancels.write().await.drain().map(|(_, t)| t).collect();
        for token in tokens {
            token.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_config_starts_with_no_backends() {
        let registry = Registry::default();
        let (sup, _rx) = Supervisor::new(McpConfig::default(), registry.clone(), FrontPeer::default());
        sup.start().await;
        assert!(sup.states().await.is_empty());
        assert!(registry.all_tools().is_empty());
    }

    #[tokio::test]
    async fn unknown_backend_has_no_peer() {
        let (sup, _rx) = Supervisor::new(McpConfig::default(), Registry::default(), FrontPeer::default());
        assert!(sup.peer("nope").await.is_none());
    }

    fn spec_from(json: &str) -> BackendSpec {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn changing_a_spawn_field_is_a_spawn_diff() {
        let a = spec_from(r#"{"command":"x","args":["1"]}"#);
        let b = spec_from(r#"{"command":"x","args":["2"]}"#);
        assert!(spawn_fields_differ(&a, &b));
    }

    #[test]
    fn changing_only_the_allowlist_is_not_a_spawn_diff() {
        let a = spec_from(r#"{"command":"x","allowedTools":["a"]}"#);
        let b = spec_from(r#"{"command":"x","allowedTools":["a","b"]}"#);
        assert!(!spawn_fields_differ(&a, &b));
        assert!(policy_differs(&a, &b));
    }

    #[test]
    fn changing_only_lazy_is_a_policy_diff() {
        let a = spec_from(r#"{"command":"x"}"#);
        let b = spec_from(r#"{"command":"x","lazy":true}"#);
        assert!(!spawn_fields_differ(&a, &b));
        assert!(policy_differs(&a, &b));
    }

    #[test]
    fn identical_specs_have_no_diff() {
        let a = spec_from(r#"{"command":"x","args":["1"],"env":{"K":"v"}}"#);
        let b = spec_from(r#"{"command":"x","args":["1"],"env":{"K":"v"}}"#);
        assert!(!spawn_fields_differ(&a, &b));
        assert!(!policy_differs(&a, &b));
    }

    #[tokio::test(start_paused = false)]
    async fn an_unspawnable_backend_reaches_failed_after_its_budget() {
        let mut cfg = McpConfig::default();
        cfg.servers.insert(
            "bad".into(),
            spec_from(r#"{"command":"definitely-not-a-real-binary-zzz"}"#),
        );
        let tuning = Tuning {
            restart_budget: 1,
            backoff_base: Duration::from_millis(1),
            backoff_ceiling: Duration::from_millis(2),
            ..Tuning::default()
        };
        let (sup, _rx) =
            Supervisor::new_tuned(cfg, Registry::default(), FrontPeer::default(), tuning);
        sup.start().await;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if matches!(sup.state("bad").await, Some(BackendState::Failed)) {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "backend never reached Failed"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}
