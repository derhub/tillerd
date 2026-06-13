## Why

The gate now binds a separate Unix socket per face — `gate-hook.sock`, `gate-tool.sock`, `gate-subscribe.sock`, `gate-admin.sock` — plus a `gate-mcp` face whose primary transport is loopback HTTP on an ephemeral TCP port published to a `gate-mcp.url` sidecar. Four of the five faces already speak the same length-prefix frame codec, and the MCP socket face already opens with a one-frame admission handshake before upgrading to the MCP protocol. The per-face split is now incidental: every gate-native producer/consumer must know a distinct well-known filename, the lone TCP port forces a non-derivable address and a stale-prone `.url` sidecar, and the MCP HTTP transport — documented as "primary, client-facing" — has zero consumers in the repo. This is the same uniformity arc that `consolidate-frame-codec` and `gate-hook-socket-ingress` walked one level lower (codec, then hook transport); the remaining step is the socket topology itself.

## What Changes

- **BREAKING** (pre-v1): the gate exposes a single `$ATHING_DIR/gate.sock`. Every connection opens with a length-prefixed **route preamble** frame — `{ route, session, token?, wireVersion }` — and the gate demultiplexes on `route` to the existing per-face behavior. The four per-face sockets and the `gate-mcp.url` sidecar are removed; the socket path is derived from `$ATHING_DIR`, never published.
- Each route preserves its current lifecycle after the preamble: `Hook` = fire-and-forget, no reply; `Tool` = request/response loop; `Subscribe` = wire-version negotiate -> ready -> server-push stream; `Admin` = request/response; `Mcp` = verify, then **upgrade** the stream to the MCP protocol (stop framing, hand the raw stream to the MCP library).
- A centralized **route -> credential policy** runs in the demux: `Hook`/`Tool`/`Mcp` require a valid session token (registry verify); `Admin` requires the admin token; `Subscribe` requires none. Admin remains separated by credential even though it is no longer separated by socket file.
- **BREAKING**: the MCP face becomes **socket-only**. The configurable HTTP transport is removed (`Transport` enum, env selectors `ATHING_GATE_MCP_TRANSPORT` / `ATHING_GATE_MCP_HTTP_PORT`, the axum app, the loopback TCP listener, `gate-mcp.url`). With the hook face already on a socket, the gate then binds **no TCP listener at all** — it is fully Unix-socket and port-free.
- The route preamble type (with a `Route` enum) is added to `contracts-rs`, shared by every gate-native client peer.
- All client peers re-wire to the single socket + preamble: the hook producer (`gate-notify`, route `Hook`), the subscribe consumers in engine and memorya (route `Subscribe`), the tool client (route `Tool`), and the orchestrator's admin client (route `Admin`).
- `docs/services.md` is rewritten to the single-socket-by-route model.

## Capabilities

### New Capabilities

- `gate-socket-multiplex`: the gate's single front-door socket. Defines the route preamble frame and its `Route` enum, the per-connection demux, the route -> credential policy, the per-route post-preamble lifecycle (including the MCP protocol upgrade), and the derived single socket path. Subsumes the previously implicit per-face socket topology for the tool, subscribe, and admin faces, which had no dedicated spec.

### Modified Capabilities

- `hook-ingress`: a hook callback arrives as the `Hook` route on the shared `gate.sock` — the preamble carries the route, session, and token in the frame envelope rather than the connection being identified by a dedicated socket file. The deterministic, restart-stable, derived-from-`$ATHING_DIR` socket-path requirement now resolves to the single `gate.sock`.
- `hook-callback-client`: the client writes the route preamble (`route: Hook`) as the opening of its connection before the hook payload frame. Fire-and-forget, runtime-free, one-shot delivery is unchanged.
- `gate-mcp-routing`: the MCP face is reached as the `Mcp` route on `gate.sock` and is **socket-only** — the configurable loopback-HTTP transport, its published `.url` endpoint, and its env selectors are removed. The face verifies the preamble's session token, then upgrades the connection to the MCP protocol exactly as the current socket transport already does.

## Impact

- **`apps/gate`**: a single `UnixListener` on `gate.sock` with a preamble reader + route demux replaces the five per-face `serve()` bindings in `service.rs`; the four framed handlers (`hook`, `tool`, `subscribe`, `admin`) are driven by the demux instead of owning their own listeners; the MCP HTTP path (`endpoint/mcp.rs` `Transport`, `http_app`, `admit`, `bind`, `mcp_url`/`write_mcp_url`) is deleted. Likely drops `axum` (and its TCP stack) from the gate crate — confirmed during tasks.
- **`contracts-rs`**: gains the route-preamble envelope type and the `Route` enum (next to `HookIngest` / `HookSubscribeRequest`), so producer, consumer, and gate share one definition.
- **Client peers re-wired**: `apps/gate-notify` (preamble `Hook`), `gate-client` + its engine/memorya subscribers (preamble `Subscribe`), the tool client (preamble `Tool`), the desktop orchestrator's admin client (preamble `Admin`); plus all integration/unit tests that connect to a per-face socket.
- **Removed discovery artifacts**: `gate-hook.sock`, `gate-tool.sock`, `gate-subscribe.sock`, `gate-admin.sock`, `gate-mcp.{sock,url}` -> one `gate.sock`. Any reader resolving those paths is updated to the single path + the route preamble.
- **Docs/ADR**: `docs/services.md` rewritten; a new ADR records the single-socket route-multiplex decision and the admin physical-wall -> credential-in-demux trust tradeoff.
- **Out of scope**: the `gate-mcp-bridge` stdio↔socket binary (the MCP analog of `gate-notify`) — deferred until an agent actually consumes the gate's own MCP tools; today none does. Also out of scope: any remote/networked gate transport, and any change to how the agent CLI fires hooks (still one process exec per event).
