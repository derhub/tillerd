# athing-mcp-gateway (Rust)

A lightweight local-first MCP gateway: a standalone, long-lived daemon that aggregates many backend
MCP servers behind a single standard MCP endpoint. Any MCP client connects to one address and reaches
every backend; one supervisor owns spawn, health, and restart. The daemon outlives the desktop UI
that launches it.

It is a sibling of the PTY daemon (`packages/daemon-pty`): same lifecycle conventions (manifest,
reuse-or-spawn, `ATHING_DIR`), no code dependency. See `docs/adr/0013-0015`.

## Toolchain

Requires a Rust toolchain (cargo). This crate lives **outside** the Bun/turbo workspace graph —
default `bun install` / `turbo run` are unaffected and do not require Rust.

```sh
cd packages/athing-mcp-gateway-rs
cargo build --release     # produces target/release/athing-mcp-gateway
cargo test                # unit + fixture + drift tests
cargo run --bin gen-schema  # regenerate schema.json from the config types
```

## Config

`~/.athing/mcp.json` (honoring `ATHING_DIR`), in the de-facto `mcpServers` format. See
`fixtures/mcp.sample.json` and `schema.json`.

```json
{
  "$schema": "./schema.json",
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"]
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "allowedTools": ["create_issue"],
      "lazy": true
    },
    "remote": { "url": "https://example.com/mcp", "headers": { "Authorization": "Bearer x" } }
  }
}
```

- `command`/`args`/`env` — process backend; `url`/`headers` — remote backend.
- `allowedTools` — expose only these (omit for all). `lazy` — defer spawn until first call (default off).
- Unknown keys inside a backend are tolerated (paste compatibility) and logged; unknown top-level keys
  are rejected. A missing file starts the gateway with no backends.

## Front and control plane

- MCP over streamable HTTP on the loopback interface, guarded by a per-launch bearer token (in the
  manifest) and a loopback-origin check.
- A REST control plane on the same server and token (health, backend status, targeted
  restart/stop/start, reload) — distinct from the MCP tool surface so the agent cannot administer the
  daemon. `GET /health` is unauthenticated (liveness + version only).

## Status

Implemented: config + schema, pure namespace router, in-memory registry. The backend connections,
supervisor, MCP face, daemon lifecycle, and control plane are tracked in
`openspec/changes/add-mcp-gateway-daemon/tasks.md`.
