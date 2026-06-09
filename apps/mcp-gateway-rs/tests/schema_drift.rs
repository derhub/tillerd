//! Schema drift guard: published schema must match generated.

#[test]
fn schema_json_matches_the_config_types() {
    let generated = athing_mcp_gateway::config_schema_json();
    let committed = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/schema.json"))
        .expect("schema.json exists — run `cargo run --bin gen-schema`");
    assert_eq!(
        generated.trim(),
        committed.trim(),
        "schema.json is stale — run `cargo run --bin gen-schema`"
    );
}
