//! The MCP face clients talk to: aggregate listings + call routing to the
//! owning backend.

use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResult, GetPromptRequestParams, GetPromptResult,
    ListPromptsResult, ListResourcesResult, ListToolsResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResult, ServerCapabilities, ServerInfo,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};

use crate::front::FrontPeer;
use crate::router;
use crate::supervisor::Supervisor;

#[derive(Clone)]
pub struct Gateway {
    supervisor: Arc<Supervisor>,
    front: FrontPeer,
}

impl Gateway {
    pub fn new(supervisor: Arc<Supervisor>, front: FrontPeer) -> Self {
        Self { supervisor, front }
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
        self.front.set(context.peer.clone());
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

        let mut params = CallToolRequestParams::new(tool.to_string());
        if let Some(args) = request.arguments {
            params = params.with_arguments(args);
        }
        peer.call_tool(params)
            .await
            .map_err(|e| Self::backend_error(backend, e))
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
