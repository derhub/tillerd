## 1. Crate scaffold

- [x] 1.1 Create `packages/athing-mcp-gateway-rs` Cargo crate (lib + daemon binary) outside the Bun/turbo graph, mirroring `packages/daemon-rs` layout and `package.json` stub
- [x] 1.2 Add dependencies: MCP Rust SDK (pinned minor) with server, client, child-process, streamable-http-client, server-side-http, macros features; async runtime; HTTP server; schema generation; error/log crates
- [x] 1.3 Add `#![deny(unsafe_code)]`, module skeleton (config, router, registry, backend, supervisor, handler, transport, daemon), and release profile matching `daemon-rs`
- [x] 1.4 Write `README.md` describing the gateway, toolchain, and how it is selected/launched

## 2. Config and schema (mcp-gateway-config)

- [x] 2.1 Implement application-data-dir resolution (`ATHING_DIR` then `~/.athing`) identical to the PTY daemon, and `mcp.json` path
- [x] 2.2 Define config types: `mcpServers` map, process vs remote backend (untagged by `command`/`url`), `allowedTools`, `lazy` default false
- [x] 2.3 Loose backend parsing (tolerate + log unknown backend keys); strict root (only `mcpServers` and `$schema`); typed load errors naming backend/field
- [x] 2.4 Missing file -> empty config (no error)
- [x] 2.5 Derive schema from config types; add a `schema.json` generator and a golden drift-test that fails on mismatch
- [x] 2.6 Write a sample `mcp.json` including a `$schema` pointer
- [x] 2.7 Golden config-parse fixtures: process backend, remote backend, allowlist, lazy, unknown-key tolerance, unknown-root rejection

## 3. Pure core (mcp-gateway-aggregation: routing + registry)

- [x] 3.1 Implement pure namespace codec (join, split-on-first-separator) with round-trip unit tests including separator-in-name
- [x] 3.2 Implement in-memory registry: namespaced index -> owner, generation counter, set/drop backend slice, allowlist filtering at index time
- [x] 3.3 Unit-test registry: generation advances on change, allowlist applied, downed backend dropped, aggregate snapshot

## 4. Backend connections (mcp-gateway-aggregation + supervision)

- [x] 4.1 Connect a process backend over child-process stdio with args/env from spec
- [x] 4.2 Connect a remote backend over streamable-http with headers from spec
- [x] 4.3 Implement a real client handler per backend that relays sampling, roots, and elicitation to the front client, and signals registry invalidation on backend list-changed
- [x] 4.4 Index a connected backend's tools/resources/prompts into the registry (namespaced, allowlist-filtered)

## 5. Supervisor (mcp-gateway-supervision)

- [x] 5.1 Backend state model (Disabled/Idle/Starting/Ready/Unhealthy/Restarting/Failed) with observable transitions
- [x] 5.2 Eager spawn + index at startup for non-lazy backends; keep warm
- [x] 5.3 Health-watch each warm backend (connection end) -> drop from index -> restart
- [x] 5.4 Active liveness ping on an interval; no response within timeout -> Unhealthy -> restart
- [x] 5.5 Capped exponential-backoff restart; exhausted budget -> terminal Failed, no further auto-restart
- [x] 5.6 Lazy: boot-index once -> idle-shutdown the process while retaining indexed primitives; respawn on first routed call with cold-start grace (await handshake before call timeout); re-index on respawn + emit list-changed; treat backend list-changed capability as a do-not-lazy hint
- [x] 5.7 Idle-shutdown timer for warm lazy backends
- [x] 5.8 Graceful shutdown: cancel all backend connections and await cleanup
- [x] 5.9 Tests: restart on exit, restart on hang, backoff cap -> Failed, lazy respawn-on-call, idle-shutdown

## 6. MCP face (mcp-gateway-aggregation: handler)

- [x] 6.1 Implement the server handler: `get_info` advertising the capability union over reachable backends
- [x] 6.2 Tools: aggregated list (namespaced, filtered) + call routing to owner; reject unknown/non-allowed with backend-tagged typed error
- [x] 6.3 Resources: aggregated list + read routing (namespaced URIs)
- [x] 6.4 Prompts: aggregated list + get routing
- [x] 6.5 Forward list-changed (tools/resources/prompts) to the front client after re-indexing; forward client cancellation to the handling backend
- [x] 6.6 Map backend errors and unavailability to typed errors that name the backend

## 7. Daemon and MCP front (mcp-gateway-daemon)

- [x] 7.1 Detach from launcher session (no orphan; not killed when launcher exits)
- [x] 7.2 Manifest `mcp-gateway.json` (pid/port/token/version): atomic write on start, remove on clean stop
- [x] 7.3 Reuse-or-spawn: connect to a live matching-version daemon via manifest; respawn on stale manifest
- [x] 7.4 Bind the MCP streamable-http endpoint to loopback only; complete handshake for standard clients
- [x] 7.5 Generate per-launch token; enforce bearer auth + loopback-origin allowlist on the MCP endpoint
- [x] 7.6 Explicit-stop only (client disconnect does not stop the daemon)

## 8. Control plane (mcp-gateway-control-plane)

- [x] 8.1 Mount REST control plane on the same loopback server/token, distinct from the tool surface
- [x] 8.2 `GET /health` unauthenticated (status + version, no sensitive data)
- [x] 8.3 `GET /backends` and `GET /backends/{name}` (name, state, pid, uptime, restart count, tool count, last error), token-guarded
- [x] 8.4 `POST /backends/{name}/restart|stop|start`: targeted, reset budget on restart, re-index, emit list-changed; stop -> Idle, start -> spawn; others untouched
- [x] 8.5 `POST /reload`: read + validate fresh config; reject invalid without disrupting running backends
- [x] 8.6 Reload diff per backend: added -> up, removed -> down, spawn-fields changed -> respawn, policy-only (allowlist/lazy) -> in-place update without respawn; serialized; best-effort per backend
- [x] 8.7 Graceful drain on disruptive change: in-flight counter, block new calls until swap, await in-flight up to drain timeout, force-cancel after timeout, serve parked calls on the new instance
- [x] 8.8 Reload returns report {added, removed, restarted, updated, unchanged, failed}
- [x] 8.9 Tests: invalid-config-safe reload, policy-only no-respawn, spawn-field respawn, partial-failure isolation, drain timeout force-cancel

## 9. Wiring and integration

- [x] 9.1 `lib.rs` public API: build gateway from config -> start supervision -> shared handler; daemon binary boots front + control plane
- [x] 9.2 Integration test: two stub backends -> aggregated list is the union, namespaced; call routes to owner
- [x] 9.3 Integration test: backend exits -> tools vanish from aggregate -> restart restores them -> list-changed emitted
- [x] 9.4 Integration test: reverse-proxy relay (a backend sampling/elicitation request reaches a stub front client and the response returns)
- [x] 9.5 `cargo test` and `cargo build --release` green; confirm pinned SDK signatures compile
