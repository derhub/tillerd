## Why

Gate has hook, tool, subscribe, and admin faces, but no MCP face — so the agent CLIs it drives cannot reach gate-hosted tools, and every future tool would otherwise reimplement the MCP wire by hand. Exposing one MCP face on gate, built on the standard Rust MCP SDK, lets any number of tools ride a single protocol layer with gate's existing auth and middleware.

## What Changes

- Add an MCP face to gate as a new transport face (`endpoint/mcp.rs`), peer to the hook and tool faces.
- Add a `Kind::Mcp` inbound variant so MCP requests route through the existing router and global middleware (auth, observe) unchanged.
- Build the face on the standard MCP Rust SDK (`rmcp`) so the MCP protocol, framing, and version negotiation are not hand-rolled — gate owns only the bridge from an MCP request to an `Inbound`.
- Support all SDK transports behind one handler: Unix socket (primary, v1), HTTP, and stdio in the first cut; TCP and WebSocket to follow. A single handler implementation serves every transport; config selects which bind.
- Authenticate every MCP request with the existing per-session bearer token, consistent with the hook and tool faces; bind loopback only (local-first, no remote surface in v1).
- Publish the bound MCP endpoint(s) and token to gate's manifest so the orchestrator can register them with the agent CLI's MCP config.

## Capabilities

### New Capabilities

- `gate-mcp-routing`: gate's MCP face — accepting MCP requests over a configurable local transport, authenticating them with the per-session token, normalizing them into `Kind::Mcp` inbounds, routing them through the existing middleware chain, and publishing the endpoint to gate's manifest for client discovery.

### Modified Capabilities

<!-- None. hook-ingress (engine hook receiver) and mcp-gateway-* (separate aggregator daemon) are
     distinct concerns and keep their current requirements. Gate's router/middleware gain a new
     route but no existing spec-level behavior changes. -->

## Impact

- **New code:** `apps/gate/src/endpoint/mcp.rs` (handler + transport selection), MCP manifest publishing.
- **Modified code:** `apps/gate/src/lib.rs` (`Kind::Mcp`), `apps/gate/src/endpoint/mod.rs` (expose module), `apps/gate/Cargo.toml` (add `rmcp`), gate service wiring (bind the face, write the manifest).
- **Dependencies:** adds `rmcp` to gate (already a vetted dependency of `mcp-gateway-rs`; pin the minor version per ADR-0015 posture).
- **Discovery contract:** the orchestrator reads gate's manifest and injects the MCP endpoint + token into the agent CLI's MCP server config; MCP clients do not auto-discover daemons.
- **Out of scope:** the tool implementations and their wiring — this change is the routing layer only.
