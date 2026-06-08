//! The MCP face clients talk to: aggregate listings + call routing to the
//! owning backend.

use std::sync::Arc;

use contracts::CorrelationId;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, GetPromptRequestParams, GetPromptResult,
    ListPromptsResult, ListResourcesResult, ListToolsResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResult, ServerCapabilities, ServerInfo,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};
use serde_json::Value;
use uuid::Uuid;

use crate::front::FrontPeer;
use crate::gate_ipc::GateToolClient;
use crate::router;
use crate::supervisor::Supervisor;

#[derive(Clone)]
pub struct Gateway {
    supervisor: Arc<Supervisor>,
    front: FrontPeer,
    gate_client: Option<Arc<GateToolClient>>,
}

impl Gateway {
    pub fn new(
        supervisor: Arc<Supervisor>,
        front: FrontPeer,
        gate_client: Option<Arc<GateToolClient>>,
    ) -> Self {
        Self {
            supervisor,
            front,
            gate_client,
        }
    }

    pub fn supervisor(&self) -> &Arc<Supervisor> {
        &self.supervisor
    }

    fn unknown(name: &str) -> McpError {
        McpError::invalid_params(format!("unknown or unavailable primitive: {name}"), None)
    }

    fn backend_error(backend: &str, e: impl std::fmt::Display) -> McpError {
        McpError::internal_error(format!("backend '{backend}': {e}"), None)
    }
}

impl ServerHandler for Gateway {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .enable_prompts()
                .enable_resources()
                .build(),
        )
        .with_instructions("Aggregating MCP gateway. Tools are namespaced as `backend__tool`.")
    }

    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        // Capture the front peer so backends can relay sampling/roots/elicitation.
        self.front.set(context.peer);
    }

    async fn list_tools(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(
            self.supervisor.registry().all_tools(),
        ))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        if self.supervisor.registry().owner_of(&request.name).is_none() {
            return Err(Self::unknown(&request.name));
        }
        let (backend, tool) =
            router::split(&request.name).ok_or_else(|| Self::unknown(&request.name))?;
        // Count this call in-flight (parks while the backend is draining).
        let _call = self.supervisor.enter_call(backend).await;
        let peer = self
            .supervisor
            .peer(backend)
            .await
            .ok_or_else(|| Self::backend_error(backend, "unavailable"))?;

        // One correlation id joins the call and its result through the gate.
        let correlation = CorrelationId(Uuid::new_v4().to_string());
        let input = request
            .arguments
            .clone()
            .map(Value::Object)
            .unwrap_or(Value::Null);
        // Observe (and, in a later firewall, gate) the call. Fail-open: the gate
        // forwards the input unchanged when it is absent or unreachable.
        let input = match &self.gate_client {
            Some(client) => client.route_call(&correlation, &request.name, input).await,
            None => input,
        };

        let mut params = CallToolRequestParams::new(tool.to_string());
        if let Some(args) = input.as_object() {
            params = params.with_arguments(args.clone());
        }
        let result = peer
            .call_tool(params)
            .await
            .map_err(|e| Self::backend_error(backend, e));

        if let (Some(client), Ok(call_result)) = (&self.gate_client, &result) {
            let rendered = serde_json::to_string(call_result).unwrap_or_default();
            client
                .observe_result(&correlation, &request.name, rendered)
                .await;
        }
        result
    }

    async fn list_prompts(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult::with_all_items(
            self.supervisor.registry().all_prompts(),
        ))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let (backend, name) =
            router::split(&request.name).ok_or_else(|| Self::unknown(&request.name))?;
        let peer = self
            .supervisor
            .peer(backend)
            .await
            .ok_or_else(|| Self::backend_error(backend, "unavailable"))?;
        let mut params = GetPromptRequestParams::new(name.to_string());
        params.arguments = request.arguments;
        peer.get_prompt(params)
            .await
            .map_err(|e| Self::backend_error(backend, e))
    }

    async fn list_resources(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult::with_all_items(
            self.supervisor.registry().all_resources(),
        ))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let backend = self
            .supervisor
            .registry()
            .resource_owner_of(&request.uri)
            .ok_or_else(|| Self::unknown(&request.uri))?;
        let peer = self
            .supervisor
            .peer(&backend)
            .await
            .ok_or_else(|| Self::backend_error(&backend, "unavailable"))?;
        peer.read_resource(ReadResourceRequestParams::new(request.uri))
            .await
            .map_err(|e| Self::backend_error(&backend, e))
    }
}
