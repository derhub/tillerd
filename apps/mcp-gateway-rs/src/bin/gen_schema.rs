//! Regenerate `schema.json` from the config types. Run with
//! `cargo run --bin gen-schema`. The golden test fails when `schema.json`
//! drifts from the types.

fn main() -> std::io::Result<()> {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schema.json");
    let mut json = tillerd_mcp_gateway::config_schema_json();
    json.push('\n');
    std::fs::write(&out, json)?;
    eprintln!("wrote {}", out.display());
    Ok(())
}
