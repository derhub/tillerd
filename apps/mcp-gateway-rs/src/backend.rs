//! Backend connection and reverse-request relay to front client.

use rmcp::model::{
    CreateElicitationRequestParams, CreateElicitationResult, CreateMessageRequestParams,
    CreateMessageResult, ListRootsResult,
};
use rmcp::service::{NotificationContext, RequestContext, RunningService};
use rmcp::transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess};
use rmcp::{ClientHandler, ErrorData as McpError, RoleClient, ServiceExt};
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;

use crate::config::{BackendKind, BackendSpec, ConfigError};
use crate::front::FrontPeer;

pub type Peer = RunningService<RoleClient, BackendHandler>;

#[derive(Clone)]
pub struct BackendHandler {
    name: String,
    front: FrontPeer,
    refresh: UnboundedSender<String>,
}

impl BackendHandler {
    pub fn new(name: String, front: FrontPeer, refresh: UnboundedSender<String>) -> Self {
        Self {
            name,
            front,
            refresh,
        }
    }

    fn no_front() -> McpError {
        McpError::internal_error("no front client connected to service this request", None)
    }
}

impl ClientHandler for BackendHandler {
    async fn create_message(
        &self,
        params: CreateMessageRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, McpError> {
        let peer = self.front.get().ok_or_else(Self::no_front)?;
        peer.create_message(params)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    async fn list_roots(
        &self,
        _context: RequestContext<RoleClient>,
    ) -> Result<ListRootsResult, McpError> {
        let peer = self.front.get().ok_or_else(Self::no_front)?;
        peer.list_roots()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    async fn create_elicitation(
        &self,
        request: CreateElicitationRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateElicitationResult, McpError> {
        let peer = self.front.get().ok_or_else(Self::no_front)?;
        peer.create_elicitation(request)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    async fn on_tool_list_changed(&self, _context: NotificationContext<RoleClient>) {
        let _ = self.refresh.send(self.name.clone());
    }

    async fn on_resource_list_changed(&self, _context: NotificationContext<RoleClient>) {
        let _ = self.refresh.send(self.name.clone());
    }

    async fn on_prompt_list_changed(&self, _context: NotificationContext<RoleClient>) {
        let _ = self.refresh.send(self.name.clone());
    }
}

// A backend that lacks prompts/resources just contributes none.
pub async fn index(
    name: &str,
    client: &rmcp::service::Peer<RoleClient>,
    registry: &crate::registry::Registry,
    allowed: Option<&[String]>,
) -> anyhow::Result<()> {
    let tools = client.list_all_tools().await?;
    let prompts = client.list_all_prompts().await.unwrap_or_default();
    let resources = client.list_all_resources().await.unwrap_or_default();
    registry.set_backend(name, tools, prompts, resources, allowed);
    Ok(())
}

pub async fn connect(
    name: &str,
    spec: &BackendSpec,
    front: FrontPeer,
    refresh: UnboundedSender<String>,
) -> anyhow::Result<Peer> {
    let kind = spec.kind(name)?;
    // The gateway spawns only external backends; first-party tools are launched by
    // the orchestrator. URL backends are connected, not spawned, so they pass.
    crate::firstparty::reject_first_party_spawn(name, kind.clone())?;
    let handler = BackendHandler::new(name.to_string(), front, refresh);
    let peer = match kind {
        BackendKind::Stdio => {
            let command = spec
                .command
                .clone()
                .ok_or_else(|| ConfigError::MissingTarget(name.to_string()))?;
            let args = spec.args.clone();
            let env = spec.env.clone();
            let cmd = Command::new(command).configure(|c| {
                c.args(&args);
                for (k, v) in &env {
                    c.env(k, v);
                }
            });
            handler.serve(TokioChildProcess::new(cmd)?).await?
        }
        BackendKind::Http => {
            let url = spec
                .url
                .clone()
                .ok_or_else(|| ConfigError::MissingTarget(name.to_string()))?;
            let transport = StreamableHttpClientTransport::from_uri(url);
            handler.serve(transport).await?
        }
    };
    Ok(peer)
}
