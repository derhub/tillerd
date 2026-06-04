//! Golden drift guard: the published `schema.json` must match the schema
//! generated from the current config types. Regenerate with
//! `cargo run --bin gen-schema` when the types change.

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
