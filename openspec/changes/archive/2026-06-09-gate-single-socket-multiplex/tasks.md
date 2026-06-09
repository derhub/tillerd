## 1. Preamble contract (contracts-rs)

- [x] 1.1 Add a `Route` enum (`Hook`, `Tool`, `Subscribe`, `Admin`, `Mcp`) and a route-preamble envelope type `{ route, session, token?, wireVersion }` next to `HookIngest` / `HookSubscribeRequest`, with serde camelCase wire shape
- [x] 1.2 Add unit + wire round-trip tests for the preamble (encode/decode each route, reject unknown route, reject unsupported wire version)
- [x] 1.3 Export the preamble type and `Route` for use by the gate and every client peer

## 2. Gate single listener + demux

- [x] 2.1 Write a failing test: connecting to `gate.sock`, sending a preamble for each route, reaches that route's behavior; a malformed/unknown/unsupported preamble is refused before any face exchange
- [x] 2.2 In `service.rs`, replace the five per-face `serve()` binds with one `UnixListener` on `base.join("gate.sock")`; per connection, read exactly one preamble frame, then demultiplex to the route handler
- [x] 2.3 Implement a centralized route→credential policy: `Hook`/`Tool`/`Mcp` require a verified session token; `Admin` requires the admin token; `Subscribe` requires none; check the credential before the route runs
- [x] 2.4 Add a negative test asserting a valid per-session token cannot satisfy the `Admin` route (no registry mutation occurs)
- [x] 2.5 Implement single-listener graceful shutdown (stop accepting + release the one socket; assert no route keeps accepting)

## 3. Route the framed faces through the demux

- [x] 3.1 Adapt the `Hook` handler to run from the demux on a post-preamble stream: fire-and-forget, no reply; session token already verified by the policy
- [x] 3.2 Adapt the `Tool` handler to the post-preamble stream: request/response loop unchanged; drop its own listener bind
- [x] 3.3 Adapt the `Subscribe` handler to the post-preamble stream: wire-version negotiate → ready → stream; preserve drop-oldest lag recording; drop its own listener bind
- [x] 3.4 Adapt the `Admin` handler to the post-preamble stream: request/response loop unchanged; admin-token credential now enforced by the policy in §2.3; drop its own listener bind

## 4. MCP route: socket-only + upgrade

- [x] 4.1 Write a failing test: an `Mcp`-route connection with a verified preamble upgrades and completes the MCP initialize handshake; an unverified preamble never upgrades
- [x] 4.2 Drive the MCP route from the demux: verify the preamble token, then hand the remaining stream to the MCP protocol library (reuse the existing socket upgrade path)
- [x] 4.3 Remove the MCP HTTP transport: the `Transport` enum + `transport_from_env`, `http_app`/`admit`, the TCP `bind`, `mcp_url`/`write_mcp_url`, and the `gate-mcp.url` sidecar
- [x] 4.4 Remove the env selectors `ATHING_GATE_MCP_TRANSPORT` and `ATHING_GATE_MCP_HTTP_PORT` and their resolution

## 5. Port-free gate cleanup

- [x] 5.1 Confirm the MCP HTTP face was the gate's last `axum`/`TcpListener` user; remove `axum` (and unused HTTP deps) from `apps/gate/Cargo.toml`
- [x] 5.2 Assert in a test that binding the gate opens no TCP port and publishes no `.url` file
- [x] 5.3 Remove all references to the per-face socket filenames (`gate-hook.sock`, `gate-tool.sock`, `gate-subscribe.sock`, `gate-admin.sock`, `gate-mcp.{sock,url}`) in gate code and tests

## 6. Re-wire client peers to the preamble

- [x] 6.1 `gate-notify`: open the connection with a `Hook` route preamble carrying session + token, then write the payload frame; update its tests
- [x] 6.2 `gate-client` + its subscribe consumers in engine and memorya: send a `Subscribe` route preamble on connect; resolve the single `gate.sock` path; update tests
- [x] 6.3 Tool client: send a `Tool` route preamble; resolve the single path; update tests
- [x] 6.4 Orchestrator admin client (desktop register/deregister): send an `Admin` route preamble with the admin token; resolve the single path; update tests
- [x] 6.5 Update every host-side resolver/env injector that pointed at a per-face socket or the MCP `.url` to resolve `$ATHING_DIR/gate.sock`

## 7. Docs + full verification

- [x] 7.1 Rewrite `docs/services.md` to the single-socket-by-route model (one `gate.sock`, the route preamble, the route→credential table; remove the per-face socket rows and `gate-mcp.url`)
- [x] 7.2 Run the full workspace test suite (Rust + Bun) and confirm green, including the new demux, credential-policy, and MCP-upgrade tests
- [x] 7.3 Manually verify a running gate binds only `gate.sock`, opens no TCP port, and serves all five routes end-to-end
