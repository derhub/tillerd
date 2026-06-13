## 1. Dependency and contract type

- [x] 1.1 Add the MCP protocol library to the gate crate manifest, feature-gated to only the transports used (server, loopback-HTTP, local socket), and pin the minor version (D1).
- [x] 1.2 Add an `Mcp` variant to the inbound `Kind` enum in `lib.rs`; confirm the workspace still builds (D2).

## 2. Route wiring

- [x] 2.1 Add a `Kind::Mcp` -> `PassThrough` entry to the router builder in `service.rs` (D3).
- [x] 2.2 Test: an authenticated `Mcp` inbound flows through the global onion (observe -> auth) and yields a terminal outcome (req: Normalization into an MCP inbound through the shared middleware).
- [x] 2.3 Test: a routed `Mcp` inbound emits exactly one observation record carrying a correlation id (req: same; ADR-0007 session-correlated observability).

## 3. MCP face — request bridge

- [x] 3.1 Add `endpoint/mcp.rs` and expose the module from `endpoint/mod.rs`.
- [x] 3.2 Implement the protocol handler: build `Inbound { kind: Kind::Mcp, session, token, body }` from an MCP request, call `Router::handle`, and map `Flow` (`Forward`/`Accepted`/`Reject`) back to an MCP result or protocol error (D1; req: Normalization into an MCP inbound).
- [x] 3.3 Expose an empty tool set so a tool listing returns empty, leaving the handler ready for tool handlers to attach later without touching auth/normalization/routing (req: Routing layer carries no tools in this version).
- [x] 3.4 Test: a compliant client completes the initialize handshake with version negotiation, and an unsupported version is declined with a protocol error (req: MCP ingress face).
- [x] 3.5 Test: a tool listing returns an empty set (req: Routing layer carries no tools).

## 4. Authentication

- [x] 4.1 Read the per-session bearer token from transport metadata — the authorization header for loopback-HTTP and the handshake for the socket — mirroring the hook face (D4; req: Per-session bearer authentication).
- [x] 4.2 Refuse a connection that presents no valid token before the protocol loop serves it (D4; req: connection refused at admission).
- [x] 4.3 Carry the token through the shared `Auth` global on every routed request so an unauthenticated request never reaches a route (D4; req: rejected before routing).
- [x] 4.4 Test: a valid token is served; a missing token is refused at admission; a wrong token is rejected as unauthenticated before routing (req: Per-session bearer authentication scenarios).

## 5. Transports

- [x] 5.1 Implement the loopback-HTTP transport bind as the primary, defaulting to an ephemeral port (D5; req: Configurable local transport, Loopback-only binding).
- [x] 5.2 Implement the local-socket transport bind as the secondary, reachable only from the same host (D5; req: Configurable local transport, Loopback-only binding).
- [x] 5.3 Select the bound transport and port from configuration in `Gate::from_env` (D7; req: Configurable local transport).
- [x] 5.4 Test: the bound network address is loopback-only, and the same authenticated request over either transport yields identical outcomes (req: Loopback-only binding, Behavior is transport-independent).

## 6. Discovery publication

- [x] 6.1 Publish the bound endpoint to a base-dir sidecar (`gate-mcp.url`) following the `gate.url` convention; do not write the per-session token to disk (D6; req: Endpoint publication for discovery without secret disclosure).
- [x] 6.2 Test: the sidecar is present and readable after binding and contains the endpoint only, with no token (req: endpoint published, discovery entry carries no token).

## 7. Service wiring and lifecycle

- [x] 7.1 Bind the MCP face in `bind_faces` beside the other faces and push its task onto the tracked task set (D7; req: MCP ingress face).
- [x] 7.2 On shutdown, abort the face task and remove the discovery sidecar (D7; ADR-0007 graceful shutdown; req: Lifecycle teardown, discovery entry cleaned up on clean shutdown).
- [x] 7.3 Test: on gate shutdown the face stops accepting new connections and the discovery sidecar is removed (req: Lifecycle teardown with the gate, discovery cleanup).

## 8. Verification

- [x] 8.1 Run the gate crate test suite, formatter, and lints; confirm green.
- [x] 8.2 Confirm the existing hook, tool, subscribe, and admin face tests still pass — the change is additive and alters no existing face behavior.
