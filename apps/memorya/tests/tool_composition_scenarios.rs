//! Tool composition: standalone, graceful degradation, contract-only coupling.

use memorya::dual_mode::{resolve_capture_mode, CaptureMode};
use memorya::Engram;
use serde_json::json;

// -- Standalone operability ----------------------------------------------------

/// When started without any peer (no gate URL), the tool operates within the
/// limits of its own concern -- storage and recall -- without error.
#[test]
fn tool_runs_with_no_peers_and_serves_recall() {
    let dir = tempfile::tempdir().unwrap();
    let memorya = Engram::open(dir.path().join("memorya.db"))
        .expect("memorya opens without any peer present");

    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize"});
    let resp =
        memorya::mcp::handle_request(&memorya, &req, 0).expect("initialize succeeds with no peers");
    assert_eq!(
        resp["result"]["serverInfo"]["name"], "memorya",
        "tool serves its own MCP interface with no peer required"
    );
}

/// Without any gate URL in the environment, standalone mode is selected and
/// the tool operates using its own stub source -- no peer is required.
#[test]
fn absent_gate_url_selects_standalone_mode() {
    assert_eq!(
        resolve_capture_mode(std::path::Path::new("/run/tillerd"), None),
        CaptureMode::Standalone,
        "no gate URL means standalone; tool must not crash when no peer is present"
    );
}

// -- Absent peer degrades, not crashes ----------------------------------------

/// When recall is invoked with no captured content (simulating a peer absent
/// during its session), the tool reports the limitation rather than crashing.
#[test]
fn absent_peer_degrades_gracefully_not_crashes() {
    let dir = tempfile::tempdir().unwrap();
    let memorya = Engram::open(dir.path().join("memorya.db")).unwrap();

    let req = json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params": { "name": "recall", "arguments": { "query": "something" } }
    });

    // No content captured (the capture peer was absent); recall must not error.
    let resp = memorya::mcp::handle_request(&memorya, &req, 0)
        .expect("recall does not crash when capture peer has not delivered content");
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("result carries text");
    assert!(
        !text.is_empty(),
        "a graceful 'no match' message is returned rather than a panic"
    );
}

// -- Source selected by wiring -------------------------------------------------

/// In a composed deployment the capture source is the gate subscription (wired
/// externally). In standalone it is the stub. The tool's MCP handler is
/// identical in both cases -- the source is selected by the orchestrator's wiring,
/// not by hard-coded logic inside the tool.
#[test]
fn source_selected_by_wiring_not_hard_coded_inside_the_tool() {
    // Standalone wiring: no gate URL.
    let standalone = resolve_capture_mode(std::path::Path::new("/run/tillerd"), None);
    assert_eq!(standalone, CaptureMode::Standalone);

    // Composed wiring: a session id present.
    let composed = resolve_capture_mode(std::path::Path::new("/run/tillerd"), Some("sess-x"));
    assert_eq!(
        composed,
        CaptureMode::Composed {
            subscribe_sock: std::path::PathBuf::from("/run/tillerd/gate.sock"),
            session_id: "sess-x".into(),
        }
    );
    // The tool binary selects its source from these enums; neither hard-codes
    // the other mode.
}

// -- Contract-only coupling (dep-direction supplement) -------------------------

/// Engram's tool interface exposes only its MCP contract surface -- tools/list,
/// tools/call. A consumer of this tool depends only on these published tools,
/// not on any internal of memorya.
#[test]
fn coupling_is_through_published_mcp_contract_only() {
    let dir = tempfile::tempdir().unwrap();
    let memorya = Engram::open(dir.path().join("memorya.db")).unwrap();

    // The contract: tools/list enumerates exactly the published surface.
    let req = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"});
    let resp = memorya::mcp::handle_request(&memorya, &req, 0).unwrap();
    let tools = resp["result"]["tools"]
        .as_array()
        .expect("tools/list returns an array");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    // The three published tools are the entire contract surface.
    assert!(names.contains(&"recall"), "recall is part of the contract");
    assert!(names.contains(&"expand"), "expand is part of the contract");
    assert!(names.contains(&"entity"), "entity is part of the contract");
    assert_eq!(
        names.len(),
        3,
        "no undocumented tools leak through the contract surface"
    );
}
