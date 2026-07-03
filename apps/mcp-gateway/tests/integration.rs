//! Integration: real backend/transport, fake client.

use std::time::Duration;

use tillerd_mcp_gateway::{build, BackendSpec, Gateway, McpConfig};

const STUB: &str = env!("CARGO_BIN_EXE_stub-backend");

fn process_spec() -> BackendSpec {
    serde_json::from_value(serde_json::json!({ "command": STUB })).unwrap()
}

fn config_with(names: &[&str]) -> McpConfig {
    let mut cfg = McpConfig::default();
    for n in names {
        cfg.servers.insert((*n).to_string(), process_spec());
    }
    cfg
}

/// Poll until the registry exposes at least `min` tools, or fail.
async fn wait_for_tools(gw: &Gateway, min: usize) {
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        if gw.supervisor().registry().all_tools().len() >= min {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "registry did not reach {min} tools in time"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn tool_names(gw: &Gateway) -> Vec<String> {
    gw.supervisor()
        .registry()
        .all_tools()
        .iter()
        .map(|t| t.name.to_string())
        .collect()
}

#[tokio::test]
async fn aggregates_a_backends_tools_under_its_namespace() {
    let gw = build(config_with(&["stub"])).await.unwrap();
    wait_for_tools(&gw, 1).await;
    let names = tool_names(&gw);
    assert!(names.contains(&"stub__echo".to_string()));
    gw.supervisor().shutdown().await;
}

#[tokio::test]
async fn routes_a_namespaced_call_to_its_backend() {
    let gw = build(config_with(&["stub"])).await.unwrap();
    wait_for_tools(&gw, 1).await;

    let peer = gw.supervisor().peer("stub").await.expect("backend ready");
    let args = serde_json::json!({ "msg": "hello-gateway" })
        .as_object()
        .unwrap()
        .clone();
    let result = peer
        .call_tool(rmcp::model::CallToolRequestParams::new("echo").with_arguments(args))
        .await
        .unwrap();

    let rendered = serde_json::to_string(&result).unwrap();
    assert!(rendered.contains("hello-gateway"), "got: {rendered}");
    gw.supervisor().shutdown().await;
}

#[tokio::test]
async fn restart_brings_a_backend_back() {
    let gw = build(config_with(&["stub"])).await.unwrap();
    wait_for_tools(&gw, 1).await;

    gw.supervisor().stop("stub").await;
    assert!(gw.supervisor().registry().all_tools().is_empty());

    gw.supervisor().restart("stub").await;
    wait_for_tools(&gw, 1).await;
    assert!(tool_names(&gw).contains(&"stub__echo".to_string()));
    gw.supervisor().shutdown().await;
}

#[tokio::test]
async fn a_lazy_backend_is_activated_on_first_call() {
    let mut cfg = McpConfig::default();
    let mut spec = process_spec();
    spec.lazy = true;
    cfg.servers.insert("lazy".into(), spec);

    let gw = build(cfg).await.unwrap();
    // Boot-index makes its tools known without staying warm.
    wait_for_tools(&gw, 1).await;

    // A lazy backend's prompt is listed without activating its process.
    let prompts: Vec<String> = gw
        .supervisor()
        .registry()
        .all_prompts()
        .iter()
        .map(|p| p.name.clone())
        .collect();
    assert!(
        prompts.contains(&"lazy__greeting".to_string()),
        "lazy backend prompt should be indexed at boot: {prompts:?}"
    );

    // Activating on demand returns a live peer.
    let peer = gw
        .supervisor()
        .peer("lazy")
        .await
        .expect("activated on demand");
    let result = peer
        .call_tool(rmcp::model::CallToolRequestParams::new("echo"))
        .await
        .unwrap();
    assert!(!result.content.is_empty());
    gw.supervisor().shutdown().await;
}

#[tokio::test]
async fn reload_diff_adds_and_removes_backends() {
    let gw = build(config_with(&["a"])).await.unwrap();
    wait_for_tools(&gw, 1).await;

    // Add b, keep a.
    let report = gw.supervisor().reload(config_with(&["a", "b"])).await;
    assert_eq!(report.added, vec!["b".to_string()]);
    assert!(report.unchanged.contains(&"a".to_string()));

    // Remove a, keep b.
    let report = gw.supervisor().reload(config_with(&["b"])).await;
    assert_eq!(report.removed, vec!["a".to_string()]);
    gw.supervisor().shutdown().await;
}
