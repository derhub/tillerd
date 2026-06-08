//! Drives every MCP server primitive against a mock service over the gate's
//! streamable-HTTP transport, asserting each round-trip by wire shape.

use rmcp::model::{
    CallToolRequestParams, CallToolResult, ClientCapabilities, ClientInfo, CompleteRequestParams,
    CompleteResult, CompletionInfo, Content, CreateMessageRequestParams, CreateMessageResult,
    GetPromptRequestParams, GetPromptResult, ListPromptsResult, ListResourceTemplatesResult,
    ListResourcesResult, ListToolsResult, PaginatedRequestParams, Prompt, PromptMessage,
    PromptMessageRole, ReadResourceRequestParams, ReadResourceResult, ResourceContents,
    SamplingMessage, ServerCapabilities, ServerInfo, SetLevelRequestParams, SubscribeRequestParams,
    Tool, UnsubscribeRequestParams,
};
use rmcp::service::RequestContext;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::{
    ClientHandler, ErrorData as McpError, RoleClient, RoleServer, ServerHandler, ServiceExt,
};

mod common;

/// A mock MCP server advertising and answering every server primitive.
#[derive(Clone)]
struct FullMcpService;

impl ServerHandler for FullMcpService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .enable_resources()
                .enable_resources_subscribe()
                .enable_logging()
                .enable_completions()
                .build(),
        )
    }

    async fn list_tools(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(vec![
            Tool::new("ping", "echoes pong", common::object_schema()),
            Tool::new(
                "sample",
                "asks the client to sample a message",
                common::object_schema(),
            ),
        ]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        match request.name.as_ref() {
            "ping" => Ok(CallToolResult::success(vec![Content::text("pong")])),
            "sample" => {
                let message = serde_json::to_value(SamplingMessage::user_text("ping")).unwrap();
                let params: CreateMessageRequestParams = serde_json::from_value(
                    serde_json::json!({ "messages": [message], "maxTokens": 16 }),
                )
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                let result = ctx
                    .peer
                    .create_message(params)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                let text = serde_json::to_string(&result).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            other => Err(McpError::invalid_params(
                format!("unknown tool: {other}"),
                None,
            )),
        }
    }

    async fn list_prompts(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult::with_all_items(vec![Prompt::new(
            "greet",
            Some("Greet the user"),
            None,
        )]))
    }

    async fn get_prompt(
        &self,
        _request: GetPromptRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            PromptMessageRole::User,
            "hello",
        )])
        .with_description("a greeting"))
    }

    async fn list_resources(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resource = serde_json::from_value(
            serde_json::json!({ "uri": "mem://greeting", "name": "greeting" }),
        )
        .unwrap();
        Ok(ListResourcesResult::with_all_items(vec![resource]))
    }

    async fn read_resource(
        &self,
        _request: ReadResourceRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            "hello",
            "mem://greeting",
        )]))
    }

    async fn list_resource_templates(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        let template = serde_json::from_value(
            serde_json::json!({ "uriTemplate": "mem://{id}", "name": "item" }),
        )
        .unwrap();
        Ok(ListResourceTemplatesResult::with_all_items(vec![template]))
    }

    async fn set_level(
        &self,
        _request: SetLevelRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        Ok(())
    }

    async fn complete(
        &self,
        _request: CompleteRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CompleteResult, McpError> {
        let completion = CompletionInfo::new(vec!["alpha".to_string(), "beta".to_string()])
            .map_err(|e| McpError::internal_error(e, None))?;
        Ok(CompleteResult::new(completion))
    }

    async fn subscribe(
        &self,
        _request: SubscribeRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        Ok(())
    }

    async fn unsubscribe(
        &self,
        _request: UnsubscribeRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        Ok(())
    }
}

/// A client that answers the server's sampling request.
#[derive(Clone)]
struct SamplingClient;

impl ClientHandler for SamplingClient {
    async fn create_message(
        &self,
        _params: CreateMessageRequestParams,
        _ctx: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, McpError> {
        Ok(CreateMessageResult::new(
            SamplingMessage::assistant_text("pong-from-client"),
            "mock-model".to_string(),
        ))
    }

    fn get_info(&self) -> ClientInfo {
        let mut info = ClientInfo::default();
        info.capabilities = ClientCapabilities::builder().enable_sampling().build();
        info
    }
}

/// Wire-shape view of a typed result, so assertions don't bind to Rust accessors.
fn json<T: serde::Serialize>(value: T) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_server_answers_every_mcp_primitive_over_the_transport() {
    let (addr, server) = common::serve(FullMcpService).await;
    let transport =
        StreamableHttpClientTransport::from_config(common::client_config(addr, None, None));
    let client = ().serve(transport).await.expect("the client completes the initialize handshake");

    let caps = json(&client.peer_info().expect("server info").capabilities);
    for capability in ["tools", "prompts", "resources", "logging", "completions"] {
        assert!(
            caps.get(capability).is_some(),
            "the server advertises the {capability} capability: {caps}"
        );
    }

    let tools = json(client.list_tools(Default::default()).await.unwrap());
    let names: Vec<&str> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(
        names.contains(&"ping") && names.contains(&"sample"),
        "{names:?}"
    );
    let called = json(
        client
            .call_tool(CallToolRequestParams::new("ping"))
            .await
            .unwrap(),
    );
    assert_eq!(called["content"][0]["text"], "pong");

    let prompts = json(client.list_prompts(Default::default()).await.unwrap());
    assert_eq!(prompts["prompts"][0]["name"], "greet");
    let prompt = json(
        client
            .get_prompt(GetPromptRequestParams::new("greet".to_string()))
            .await
            .unwrap(),
    );
    assert_eq!(prompt["messages"][0]["content"]["text"], "hello");

    let resources = json(client.list_resources(Default::default()).await.unwrap());
    assert_eq!(resources["resources"][0]["uri"], "mem://greeting");
    let read = json(
        client
            .read_resource(ReadResourceRequestParams::new("mem://greeting".to_string()))
            .await
            .unwrap(),
    );
    assert_eq!(read["contents"][0]["text"], "hello");
    let templates = json(
        client
            .list_resource_templates(Default::default())
            .await
            .unwrap(),
    );
    assert_eq!(
        templates["resourceTemplates"][0]["uriTemplate"],
        "mem://{id}"
    );

    client
        .set_level(serde_json::from_value(serde_json::json!({ "level": "info" })).unwrap())
        .await
        .unwrap();

    let complete = json(
        client
            .complete(
                serde_json::from_value(serde_json::json!({
                    "ref": { "type": "ref/prompt", "name": "greet" },
                    "argument": { "name": "x", "value": "al" },
                }))
                .unwrap(),
            )
            .await
            .unwrap(),
    );
    assert_eq!(
        complete["completion"]["values"],
        serde_json::json!(["alpha", "beta"])
    );

    client
        .subscribe(serde_json::from_value(serde_json::json!({ "uri": "mem://greeting" })).unwrap())
        .await
        .unwrap();
    client
        .unsubscribe(
            serde_json::from_value(serde_json::json!({ "uri": "mem://greeting" })).unwrap(),
        )
        .await
        .unwrap();

    let _ = client.cancel().await;
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_server_requests_sampling_from_the_connected_client() {
    let (addr, server) = common::serve(FullMcpService).await;
    let transport =
        StreamableHttpClientTransport::from_config(common::client_config(addr, None, None));
    let client = SamplingClient
        .serve(transport)
        .await
        .expect("the sampling client completes the initialize handshake");

    let result = json(
        client
            .call_tool(CallToolRequestParams::new("sample"))
            .await
            .unwrap(),
    );
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("pong-from-client"),
        "the server relayed the client's sampled message: {text}"
    );

    let _ = client.cancel().await;
    server.abort();
}
