//! Minimal MCP server used as a real backend in integration tests. Exposes one
//! tool, `echo`, that returns its arguments as text. Speaks MCP over stdio.

use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListPromptsResult,
    ListToolsResult, PaginatedRequestParams, Prompt, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::transport::io::stdio;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, ServiceExt};

#[derive(Clone)]
struct Stub;

fn object_schema() -> Arc<serde_json::Map<String, serde_json::Value>> {
    let mut s = serde_json::Map::new();
    s.insert("type".into(), serde_json::Value::String("object".into()));
    Arc::new(s)
}

impl ServerHandler for Stub {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
        )
    }

    async fn list_prompts(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        let greeting: Prompt =
            serde_json::from_value(serde_json::json!({ "name": "greeting" })).unwrap();
        Ok(ListPromptsResult::with_all_items(vec![greeting]))
    }

    async fn list_tools(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(vec![Tool::new(
            "echo",
            "Echo the arguments back as text",
            object_schema(),
        )]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        match request.name.as_ref() {
            "echo" => {
                let text = serde_json::to_string(&request.arguments).unwrap_or_default();
                Ok(CallToolResult::success(vec![ContentBlock::text(text)]).into())
            }
            other => Err(McpError::invalid_params(
                format!("unknown tool: {other}"),
                None,
            )),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = Stub.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
