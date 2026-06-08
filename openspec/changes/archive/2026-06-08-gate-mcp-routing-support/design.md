## Context

Gate is a composable middleware core (`apps/gate`) with four transport faces wired in
`service.rs`: a hook ingress (loopback HTTP via `axum`), and three loopback Unix-socket
faces (tool, subscribe, admin) using a 4-byte length-prefix frame codec. Every face
funnels an `Inbound { kind, session, correlation, token, body }` into a single
`Router::handle`, which mints a correlation id when absent and runs the global onion
(`Observe`, then `Auth`) wrapped around a per-`Kind` route. `Kind` today is
`Hook | ToolCall | ToolResult`; the tool routes are `PassThrough` (forward the body
unchanged). Auth is a per-session bearer `Token` verified in constant time against the
`SessionRegistry`; sessions are minted out of band via the admin face.

The gate runs as a `service-host` `Service`. `service-host` owns the manifest, but that
manifest (`ManifestData`) carries only `{ pid, version }` — it is a liveness/identity
record, **not** an endpoint directory. Faces publish their reachable address as a sidecar
file in the resolved base dir (`~/.athing` by default): the hook face writes
`<base>/gate.url`; the IPC faces bind `<base>/gate-<face>.sock`. The orchestrator reads
those sidecars to wire the agent CLI.

This change adds a fifth face — MCP — so the agent CLI gate drives can reach gate-hosted
tools over one standard protocol, reusing the existing router, auth, and observation
unchanged. See `proposal.md` for motivation. Scope here is the routing layer only: the
plumbing that accepts MCP, authenticates it, routes it as a new `Kind`, and publishes the
endpoint. Tool implementations are a later change that attaches handlers to this layer.

## Goals / Non-Goals

**Goals:**

- One MCP face (`endpoint/mcp.rs`), peer to the existing faces, built on the standard
  Rust MCP SDK (`rmcp`) so protocol framing, the `initialize` handshake, and version
  negotiation are not hand-rolled.
- A `Kind::Mcp` inbound that flows through the unchanged global onion (`Observe`, `Auth`)
  and a per-kind route, so authentication and observation apply identically to MCP.
- Per-session bearer-token auth, consistent with the hook and tool faces; loopback bind
  only (no remote surface in v1).
- Publish the bound MCP endpoint as a base-dir sidecar so the orchestrator can inject it
  into the agent CLI's MCP server config, following the `gate.url` precedent.

**Non-Goals:**

- Tool implementations and their wiring (the `tools/list` / `tools/call` payloads). The
  v1 face exposes an empty tool set; real handlers route through this layer in a later
  change.
- Remote transports and any non-loopback exposure.
- Multi-user / multi-tenant auth. One subscription = one user holds.
- A new generic endpoint format in the `service-host` manifest. Endpoints stay sidecar
  files, as today.

## Decisions

### 1. Build the face on `rmcp`, own only the request→`Inbound` bridge

Gate implements an `rmcp` `ServerHandler`; `rmcp` owns transport, JSON-RPC 2.0 framing,
the `initialize`/capability handshake, and version negotiation. The handler's tool-call
path is the only gate-owned code: it constructs `Inbound { kind: Kind::Mcp, .. }` and
calls `Router::handle`, then maps the `Flow` back to an MCP result.

- *Why:* the MCP wire is non-trivial and evolving; reimplementing it per future tool is
  exactly what the proposal exists to prevent. `rmcp` is already a vetted dependency of
  the separate `mcp-gateway-rs` aggregator.
- *Alternative — hand-roll JSON-RPC over the existing frame codec:* rejected. The codec
  carries opaque length-prefixed bytes; MCP needs request/response correlation, the
  initialize lifecycle, and capability negotiation on top. That is a protocol library, not
  a framing tweak.

### 2. Add a single `Kind::Mcp` variant (not per-method)

`Kind` gains one variant, `Mcp`. The MCP method (`tools/call`, etc.) lives inside the
body; routing is by face, and method-level dispatch belongs to the `rmcp` handler above
the route, not to the router.

- *Why:* mirrors how the hook face carries the hook event in the body and lets the
  `Normalize` layer interpret it. The router stays a thin kind→chain dispatcher.
- *Alternative — `McpCall` / `McpList` / … variants:* rejected; it pushes protocol
  knowledge into the router and multiplies routes for no routing benefit. Contrast with
  `ToolCall`/`ToolResult`, which are split because they are genuinely different inbound
  directions, not different methods.

### 3. v1 route for `Kind::Mcp` is `PassThrough`

In `build_router`, `Kind::Mcp` maps to `PassThrough`, exactly like the tool routes. The
face authenticates and normalizes; the route forwards. Real tool dispatch attaches later
by replacing `PassThrough` with a tool-dispatch middleware (or by the handler resolving
tools before it calls `Router::handle`).

- *Why:* the deliverable now is authenticated, observed, routed plumbing — not tools. An
  empty tool set means `tools/list` returns `[]` and the forward path is exercised by
  tests until tools land.
- *Alternative — block this change until tools exist:* rejected; the proposal explicitly
  sequences routing first so tools ride a ready protocol layer.

### 4. Auth at two points: connection admission and per-request routing

The bearer token is read from transport metadata (HTTP `Authorization` header for the
streamable-HTTP transport; a handshake frame for the socket transport), exactly as the
hook face reads `Authorization`. The face rejects an unauthenticated connection before
serving the `rmcp` loop, **and** every routed call still carries the `Token` through the
router's `Auth` global.

- *Why:* connection admission fails fast and keeps unauthenticated peers off the protocol
  loop; per-request auth keeps MCP identical to every other kind and is the single source
  of truth (`Reject::Unauthenticated`). Defense in depth at no extra contract surface.
- *Alternative — auth only at the router:* workable but serves the full handshake to
  anonymous peers first. *Alternative — auth only at the connection:* breaks the
  invariant that every `Kind` is authorized by the same `Auth` global.

### 5. Streamable HTTP is the v1 client-facing transport; Unix socket secondary

The agent CLI is the MCP client (per the proposal's discovery contract). An MCP client
dials either a spawned stdio command or an HTTP(S) URL — it cannot dial a Unix socket. A
singleton daemon also cannot own one stdin/stdout per client. Therefore v1 binds
`rmcp`'s streamable-HTTP server on loopback and publishes `<base>/gate-mcp.url`, reusing
the hook face's loopback + bearer posture. The Unix-socket transport (`rmcp` IO transport
over `UnixStream`, published as `<base>/gate-mcp.sock`) ships as a secondary for
in-process / orchestrator clients that can dial it directly.

- *Why:* HTTP-on-loopback is the only transport an unmodified agent-CLI MCP client can
  reach against a running daemon, and it inherits gate's existing HTTP auth shape.
- *This refines the proposal's "Unix socket primary, stdio in the first cut" ordering.*
  Unix socket as the *primary client* path needs a stdio↔socket bridge binary the CLI
  spawns; stdio direct needs gate-as-subprocess, which contradicts the singleton daemon.
  Both are deferred — see Open Questions. TCP and WebSocket remain post-v1 as the proposal
  states.

### 6. Publish the endpoint as a base-dir sidecar, not in the manifest, and without the token

The face writes the bound endpoint to `<base>/gate-mcp.url` (and `<base>/gate-mcp.sock`
when the socket transport is enabled), mirroring `hook::write_gate_url`. The
`service-host` manifest is left as `{ pid, version }`.

- *Why:* the manifest is an identity/liveness record; endpoints are already sidecars.
  Consistency beats inventing a manifest endpoint schema.
- *The per-session token is not written to disk.* It is per-session and the orchestrator
  already holds it (it minted the session via the admin face); it injects token + endpoint
  into the CLI config together. This corrects the implementation-plan sketch that wrote a
  shared `gate.json` carrying the token. Endpoint discovery and secret distribution stay
  separate, and no long-lived secret lands on disk.

### 7. Config-selected binds, `Gate::from_env`

Transport selection and the loopback HTTP port come from the environment, resolved in
`Gate::from_env` and bound in `bind_faces` alongside the other faces (e.g.
`ATHING_GATE_MCP_HTTP_PORT`, ephemeral `0` default, matching `hook::port_from_env`). A
new face task is pushed onto `self.tasks` so existing shutdown (`task.abort()`) tears it
down with the rest.

- *Why:* one wiring site, one teardown path, no new lifecycle machinery.

## Risks / Trade-offs

- **`rmcp` API drift** → pin the minor version per the repo's dependency-pinning posture;
  confine all `rmcp` types to `endpoint/mcp.rs` so an upgrade touches one module. The
  router, `Kind`, and middleware never name an `rmcp` type.
- **`rmcp` owns its own transport/runtime; gate owns the other faces' I/O** → keep the
  boundary at "`rmcp` serves the protocol, gate's handler builds the `Inbound`." Do not
  thread the gate frame codec into MCP, and do not let `rmcp` reach the registry except
  through the `Token` on the `Inbound`.
- **Empty tool set in v1 is a near-no-op surface** → acceptable and intended; cover it
  with tests that prove a `tools/call` is authenticated, routed as `Kind::Mcp`, observed
  once, and forwarded, plus an unauthenticated call rejected before routing. The contract
  is exercised even with zero tools.
- **Binary size / cold start** → `rmcp` pulls a transport stack into the gate binary;
  gate the HTTP-server feature behind the features actually used (`server`,
  streamable-HTTP, and IO for the socket) and avoid unused transports.
- **Loopback-only is the security boundary** → all binds are `127.0.0.1` / Unix socket
  with filesystem permissions; no remote listener exists to misconfigure in v1.

## Migration Plan

Additive. No existing behavior or contract changes; `Kind` gains a variant,
`build_router` gains one route entry, `bind_faces` gains one task, and `Cargo.toml` gains
`rmcp`. The face only binds when its config enables it, so the gate runs unchanged until
the orchestrator opts a session into MCP. Rollback is removing the face task and the route
entry — no persisted state, no schema, the sidecar file is recreated each boot and removed
on clean shutdown like the other endpoint files.

## Open Questions

- **Stdio reachability for the agent CLI.** If the target CLI only supports stdio MCP
  servers, a thin `stdio↔gate-mcp.sock` bridge binary (the CLI spawns it; it dials the
  socket) is the bridge. Is that bridge in scope for a follow-up change, or does the v1
  HTTP transport cover the v1 CLI? Resolve before wiring orchestrator injection.
- **ADR.** This adds a new external dependency and a new architectural face. The change
  uses the `spec-driven` schema (no ADR artifact). Confirm whether the `rmcp` adoption +
  MCP face warrant a standalone ADR under `docs/adr/` per the repo's pattern, and the
  correct number, rather than recording the dependency-pinning rationale only here.
- **Session correlation across an MCP connection.** A streamable-HTTP MCP connection is
  long-lived and multiplexes calls. Confirm the session id binds at connection admission
  (from the URL/header the orchestrator injects) and is stamped onto every `Inbound` for
  that connection, so per-call observation stays session-correlated.
