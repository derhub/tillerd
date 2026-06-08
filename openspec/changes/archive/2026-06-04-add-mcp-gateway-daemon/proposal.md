## Why

Users accumulate many MCP servers (typically 3-8, power users 10-50), and every MCP client must
be configured with the full list independently — duplicating credentials, spawn commands, and
process supervision in each client. A single aggregating gateway lets any MCP client connect to
one endpoint and reach every backend, while one supervisor owns spawn, health, and restart. The
gateway must outlive the desktop UI so background servers and their warm state survive the UI
closing — so it is a standalone long-lived daemon, a sibling to the existing PTY daemon, not an
in-process component.

## What Changes

- New standalone Rust daemon crate `packages/athing-mcp-gateway-rs`, outside the Bun/turbo graph
  (same posture as `packages/daemon-pty`), built on `rmcp` 1.7.
- New config file `~/.athing/mcp.json` (honoring `ATHING_DIR`) in the de-facto `mcpServers`
  format, with per-backend `allowedTools` (allowlist) and `lazy` extensions, plus a generated
  `schema.json` and `$schema` pointer for editor validation.
- The daemon aggregates many backend MCP servers (stdio and streamable-http) behind one MCP face
  and exposes them over **standard streamable-http** on loopback, guarded by a bearer token and
  an origin allowlist. No bespoke wire protocol — every consumer (the desktop UI, third-party MCP
  clients) connects as an ordinary MCP client.
- Full bidirectional aggregation: tools, resources, and prompts forward both ways, including the
  reverse-direction server->client requests (sampling, roots, elicitation) and `list_changed` /
  cancellation notifications.
- A supervisor with a per-backend state model, crash + hang healing (active liveness ping),
  capped-backoff restart, lazy spawn with idle-shutdown, and graceful drain.
- A token-guarded REST control plane on the same loopback server for health, per-backend status,
  targeted restart/stop/start, and a `/reload` that diffs `mcp.json` and applies changes with
  graceful drain — distinct from the MCP tool surface so the agent cannot administer the daemon.
- Daemon lifecycle: manifest `~/.athing/mcp-gateway.json` (`pid`/`port`/`token`/`version`),
  atomic write, detach, and reuse-or-spawn — mirroring the PTY daemon's conventions without
  depending on its code.

## Capabilities

### New Capabilities

- `mcp-gateway-config`: the `mcp.json` config contract — location/resolution, `mcpServers`
  format, `allowedTools` and `lazy` per-backend extensions, loose-strictness paste compatibility,
  and the schemars-generated `schema.json` with drift protection.
- `mcp-gateway-aggregation`: presenting many backends as one MCP server — tool/resource/prompt
  namespacing and routing, capability union, the in-memory registry with generation invalidation
  and allowlist filtering, the full reverse-proxy relay (sampling/roots/elicitation), and
  notification forwarding.
- `mcp-gateway-supervision`: backend process lifecycle — the state model, eager and lazy spawn,
  health-watch, active-liveness healing, capped-backoff restart to a terminal Failed state, and
  idle-shutdown.
- `mcp-gateway-daemon`: the daemon and its MCP front — manifest, detach, reuse-or-spawn, explicit
  stop, and the streamable-http endpoint with bearer-token and origin-allowlist authentication.
- `mcp-gateway-control-plane`: the REST management surface — health and status endpoints,
  targeted restart/stop/start, and `/reload` with per-backend diff and graceful drain.

### Modified Capabilities

<!-- None. The gateway is an additive sibling daemon; it consumes no existing capability's
     requirements and the PTY daemon's wire protocol is untouched. -->

## Impact

- New crate `packages/athing-mcp-gateway-rs` (lib + daemon binary), requiring a Rust toolchain;
  outside default `bun install` / `turbo run`, like `packages/daemon-pty`.
- New runtime config and state files under `~/.athing/`: `mcp.json` (user-authored),
  `mcp-gateway.json` (manifest), and `schema.json` (shipped).
- New dependency surface: `rmcp` 1.7 (`server`, `client`, `transport-child-process`,
  `transport-streamable-http-client`, `server-side-http`, `macros`), `tokio`, `axum`, `schemars`.
- New ADR(s) under `docs/adr/` capturing the standalone-daemon, MCP-only-front, and
  Rust/`rmcp` decisions (continuing the 0008-0011 daemon series).
- The desktop app gains a second background daemon to spawn and connect to (as an MCP client);
  the PTY daemon and its IPC are unaffected.
- Platform scope macOS/Linux for v1.
