//! Golden config-parse fixture: the sample `mcp.json` parses to the expected
//! shape. The fixture is the contract for the de-facto config format.

use athing_mcp_gateway::config::{BackendKind, McpConfig};

fn sample() -> McpConfig {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/mcp.sample.json"
    ))
    .expect("sample fixture exists");
    McpConfig::from_json(&raw).expect("sample parses")
}

#[test]
fn sample_has_three_backends() {
    assert_eq!(sample().servers.len(), 3);
}

#[test]
fn sample_validates() {
    sample().validate().expect("sample is valid");
}

#[test]
fn filesystem_is_a_process_backend() {
    let cfg = sample();
    let fs = &cfg.servers["filesystem"];
    assert_eq!(fs.kind("filesystem").unwrap(), BackendKind::Stdio);
    assert_eq!(fs.command.as_deref(), Some("npx"));
}

#[test]
fn github_carries_allowlist_and_lazy() {
    let cfg = sample();
    let gh = &cfg.servers["github"];
    assert!(gh.lazy);
    assert!(gh.allows_tool("create_issue"));
    assert!(!gh.allows_tool("delete_repo"));
}

#[test]
fn remote_is_a_http_backend() {
    let cfg = sample();
    let r = &cfg.servers["remote"];
    assert_eq!(r.kind("remote").unwrap(), BackendKind::Http);
    assert_eq!(r.url.as_deref(), Some("https://example.com/mcp"));
}

#[test]
fn schema_pointer_is_accepted() {
    assert!(sample().schema.is_some());
}
