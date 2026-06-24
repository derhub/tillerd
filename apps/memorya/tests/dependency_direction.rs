//! Dependency guards: tools don't depend on each other or orchestrator.
//! - Only `daemon-pty-client` (and the daemon itself) know the PTY wire.
//! - `gate-client` and `contracts` carry no tool deps.
//! - The daemon depends on nothing downstream (no gate / gateway / memorya).
//!
//! Engram-scoped rules (subset of the above, included here for locality):
//! - memorya never pulls the daemon, the PTY client, or an async socket stack.
//! - gate-client never pulls memorya or the PTY client.
//! - The only gate-wire crates memorya touches are the contract types and the codec.
//!
//! Asserted against `cargo metadata`'s resolved graph so a stray edge fails CI
//! rather than silently inverting the architecture.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::process::Command;

use serde_json::Value;

fn metadata() -> Value {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--locked"])
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata emits json")
}

fn names_by_id(meta: &Value) -> HashMap<String, String> {
    meta["packages"]
        .as_array()
        .expect("metadata has packages")
        .iter()
        .map(|p| {
            (
                p["id"].as_str().expect("package id").to_string(),
                p["name"].as_str().expect("package name").to_string(),
            )
        })
        .collect()
}

fn id_for(meta: &Value, name: &str) -> String {
    meta["packages"]
        .as_array()
        .expect("metadata has packages")
        .iter()
        .find(|p| p["name"].as_str() == Some(name))
        .and_then(|p| p["id"].as_str())
        .unwrap_or_else(|| panic!("{name} is not a workspace package"))
        .to_string()
}

/// Crate names in `root`'s transitive normal-dependency closure: normal edges
/// only (dev- and build-dependencies excluded), `root` itself excluded.
fn normal_closure(meta: &Value, root: &str) -> BTreeSet<String> {
    let names = names_by_id(meta);
    let nodes: HashMap<&str, &Value> = meta["resolve"]["nodes"]
        .as_array()
        .expect("resolve has nodes")
        .iter()
        .map(|n| (n["id"].as_str().expect("node id"), n))
        .collect();

    let root_id = id_for(meta, root);
    let mut visited: HashSet<String> = HashSet::from([root_id.clone()]);
    let mut frontier = vec![root_id];
    let mut closure = BTreeSet::new();

    while let Some(id) = frontier.pop() {
        let node = nodes.get(id.as_str()).expect("every id has a resolve node");
        for dep in node["deps"].as_array().into_iter().flatten() {
            let is_normal = dep["dep_kinds"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|k| k["kind"].is_null()));
            if !is_normal {
                continue;
            }
            let pkg = dep["pkg"].as_str().expect("dep has pkg id").to_string();
            if visited.insert(pkg.clone()) {
                if let Some(name) = names.get(&pkg) {
                    closure.insert(name.clone());
                }
                frontier.push(pkg);
            }
        }
    }
    closure
}

fn leaked<'a>(closure: &BTreeSet<String>, forbidden: &[&'a str]) -> Vec<&'a str> {
    forbidden
        .iter()
        .copied()
        .filter(|c| closure.contains(*c))
        .collect()
}

#[test]
fn memorya_lib_has_no_daemon_pty_or_socket_dependency() {
    let closure = normal_closure(&metadata(), "memorya");
    // daemon -> the daemon crate; pty -> the daemon PTY client; socket -> the
    // async socket stack memorya (sync, blocking std sockets) must never pull in.
    let forbidden = [
        "tillerd",
        "tillerd-daemon-pty-client",
        "tokio",
        "mio",
        "socket2",
    ];

    assert_eq!(
        leaked(&closure, &forbidden),
        Vec::<&str>::new(),
        "memorya must not depend on the daemon, the PTY client, or an async socket stack"
    );
}

#[test]
fn gate_client_has_no_memorya_or_daemon_pty_client_dependency() {
    let closure = normal_closure(&metadata(), "tillerd-gate-client");
    let forbidden = ["memorya", "tillerd-daemon-pty-client"];

    assert_eq!(
        leaked(&closure, &forbidden),
        Vec::<&str>::new(),
        "gate-client must not cycle back to memorya nor reach for the PTY client's wire"
    );
}

#[test]
fn memorya_wire_dependencies_are_only_contracts_rs_and_gate_client() {
    let closure = normal_closure(&metadata(), "memorya");
    let wire_family: BTreeSet<&str> = [
        "tillerd-contracts",
        "tillerd-gate-client",
        "tillerd-daemon-pty-client",
        "tillerd",
        "tillerd-gate",
    ]
    .into();

    let wire_deps: BTreeSet<&str> = closure
        .iter()
        .map(String::as_str)
        .filter(|c| wire_family.contains(c))
        .collect();

    assert_eq!(
        wire_deps,
        BTreeSet::from(["tillerd-contracts", "tillerd-gate-client"]),
        "memorya's only gate-wire deps must be the contract types and the codec"
    );
}

#[test]
fn gate_has_no_memorya_or_gateway_or_pty_client_dependency() {
    let meta = metadata();
    let closure = normal_closure(&meta, "tillerd-gate");
    let forbidden = [
        "memorya",
        "tillerd-mcp-gateway",
        "tillerd-daemon-pty-client",
    ];

    assert_eq!(
        leaked(&closure, &forbidden),
        Vec::<&str>::new(),
        "tillerd-gate must not depend on memorya, the mcp-gateway, or the PTY client"
    );
}

#[test]
fn gateway_has_no_memorya_or_gate_or_daemon_dependency() {
    let meta = metadata();
    let closure = normal_closure(&meta, "tillerd-mcp-gateway");
    let forbidden = [
        "memorya",
        "tillerd-gate",
        "tillerd",
        "tillerd-daemon-pty-client",
    ];

    assert_eq!(
        leaked(&closure, &forbidden),
        Vec::<&str>::new(),
        "tillerd-mcp-gateway must not depend on memorya, the gate process, the daemon, or the PTY client"
    );
}

#[test]
fn daemon_has_no_downstream_tool_dependency() {
    let meta = metadata();
    let closure = normal_closure(&meta, "tillerd");
    let forbidden = [
        "tillerd-gate",
        "tillerd-mcp-gateway",
        "memorya",
        "tillerd-gate-client",
    ];

    assert_eq!(
        leaked(&closure, &forbidden),
        Vec::<&str>::new(),
        "tillerd-daemon-pty must not depend on any downstream tool (gate / gateway / memorya)"
    );
}

#[test]
fn contracts_rs_has_no_tool_dependency() {
    let meta = metadata();
    let closure = normal_closure(&meta, "tillerd-contracts");
    let forbidden = [
        "memorya",
        "tillerd-gate",
        "tillerd-mcp-gateway",
        "tillerd",
        "tillerd-gate-client",
        "tillerd-daemon-pty-client",
    ];

    assert_eq!(
        leaked(&closure, &forbidden),
        Vec::<&str>::new(),
        "contracts-rs must be a pure leaf — no tool dependencies"
    );
}

#[test]
fn only_daemon_and_pty_client_know_the_pty_wire() {
    // gate, gateway, and memorya must not pull in daemon-pty-client transitively.
    let meta = metadata();
    let tools = ["tillerd-gate", "tillerd-mcp-gateway", "memorya"];

    for tool in tools {
        let closure = normal_closure(&meta, tool);
        assert_eq!(
            leaked(&closure, &["tillerd-daemon-pty-client"]),
            Vec::<&str>::new(),
            "{tool} must not reach tillerd-daemon-pty-client (PTY wire is daemon-only)"
        );
    }
}
