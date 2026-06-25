## Why

Client-provided streaming already works in tillerd — but only as a bespoke one-off for surface (terminal) output: `surface_create` is hand-written because a `tauri::ipc::Channel` cannot live in the core bus, `ChannelSink` is Tauri-specific (no server-host adapter), and `makeSurfaceChannel` is a bespoke client binding. There is no reusable `subscribe`/`stream` verb, no host-agnostic sink port, and no transport macro — so the next stream (logs tail, task progress, supervision) must re-hand-roll the whole path, and the expected web/server host has no way to serve streams at all.

This change generalizes the proven surface-streaming pattern into a reusable, host-agnostic `subscribe`/`stream` capability, then migrates surface output onto it to validate the abstraction against the real high-rate case.

## What Changes

- Add a **host-agnostic sink-registration port**: a subscription registers a client-provided sink (desktop `ipc::Channel`, server WebSocket) into a domain `Broadcast`, scoped by a key (surface/session id). Generalizes `ChannelSink` + the `SurfaceChannels` registry; the orchestrator core speaks a sink trait, never a Tauri type.
- **`subscribe` is a tower-observed bus command**, not a `Service`-returns-`Stream`: the subscribe call carries + registers the client sink and returns; frames then flow **zero-copy through the registered `tx`**, not re-dispatched through `bus.execute` per frame. Middleware (logging, future auth/rate-limit) applies per *subscription setup*, never per streamed item. Research basis: the tower-standard `Service`→`Stream` (tonic/axum SSE) is network-transport-shaped, requires owned stream items (breaks ADR-0037 zero-copy), and Tauri commands cannot return a stream anyway — so the core stays sink-port + `Broadcast`, and the server host may expose `Service`→`Stream`→SSE/WebSocket at *its own* adapter edge, fed by the same port.
- Add a **`transport_subscribe!` macro** so a streaming endpoint is declared like `transport_command!`/`transport_query!`, not hand-written. It accepts the client sink + params, dispatches a `Subscribe` command, and registers the sink.
- Add a **generic `subscribe`/`stream` client binding** generalizing `makeSurfaceChannel` (typed channel creation + lifecycle) in `@tillerd/client-bindings`.
- **Migrate surface output** onto the generic mechanism: `surface_create`/attach/detach become the `transport_subscribe!` form over the host-agnostic port; the bespoke `ChannelSink`/`SurfaceChannels` collapse into the generic sink registry. Behavior unchanged (zero-copy byte path preserved).
- **BREAKING (pre-v1, new seam):** the surface streaming transport shape changes (generic subscribe replaces the bespoke command). Internal/IPC seam; the renderer byte path (`channel.onmessage`) is preserved.

## Out of scope (follow-on, with locked decisions recorded)

- **New stream endpoints (logs follow, task progress, supervision)** — they consume the new mechanism but ship separately. The first consumer migrated here is surface output (PTY bytes over `ipc::Channel`); it touches no files, so the file-I/O work below is NOT in this change.
- **Logs-stream follow-on (locked decisions):** a `log_subscribe` (live `tail -f`) endpoint SHALL use **`notify` directly** (Windows-safe via ReadDirectoryChangesW; not `linemux`, which pins old notify and is lightly maintained) — `notify` event -> `spawn_blocking` seek-from-last-offset read -> push to the client sink. Distinct from the existing one-shot `log_tail` (last-N-lines windowed read).
- **Non-blocking-I/O sweep (locked decisions, separate change):** orchestrator handlers do synchronous `std::fs` in async bodies (`shared/fs.rs`, `infra/config/*`, `app/template/*`) — they block the tokio worker. Fix: **`tokio::fs`** for one-shot config/template reads/writes; **one `spawn_blocking`** wrapping `fs::tail`'s windowed read loop (`tokio::fs` is itself `spawn_blocking` per call — chunked loops cost a hop per line, which tokio's own docs warn against). Required before the logs-stream follow-on; the config writes are microsecond one-shots so lower urgency.
- **The server/web host adapter implementation** (the port makes it possible; building it is later). The server edge MAY expose a tower `Service`->`Stream`->SSE/WebSocket, fed by the same sink port.

## Locked design decisions (this change)

- `subscribe` = a tower-observed bus **command** that carries the client-provided `tx` (sink) and registers it (tower middleware wraps the subscribe call). Once registered, **frames flow zero-copy from the source straight through the registered `tx`** — they are NOT re-dispatched through `bus.execute` per frame (that would force an owned copy + a pipeline pass on a high-rate stream). The bus owns the subscription; the `tx` is the frame channel. NOT a `Service`->`Stream` (owned items break zero-copy; Tauri can't return streams). Per-frame middleware, if ever needed, is the ADR-0037 sink-wrapping tier (borrowed, zero-copy), not the tower bus.
- **`command`/`query` keep returning values** (Tauri Promise / HTTP body) — a client `tx` is for streaming (N items + backpressure), not for non-blocking, and not for unary calls. The arity split (0/1/N values) maps cleanly to both desktop and the future server host.
- **Two middleware tiers:** subscribe setup -> tower Layer (logging/auth); per-item stream -> ADR-0037 sink-wrapping (sync, borrowed, zero-copy). Per-item is NOT a tower Service.
- **Client owns its `tx`** (`ipc::Channel` desktop / WebSocket server) and its own handling; the host-agnostic sink port is the only host-specific seam.

## Capabilities

### New Capabilities

- `stream-subscription`: the host-agnostic subscribe/stream mechanism — the sink-registration port, the `subscribe` bus command + lifecycle (subscribe → stream → drop/unsubscribe), key-scoped routing, and the transport-macro contract. Defines what a subscription is, how a client sink is provided and torn down, and the boundary between on-bus setup and off-bus streaming.

### Modified Capabilities

- `event-dispatch`: generalize the borrowed-event/`Broadcast`/sink standard to cover key-scoped client-sink subscriptions (registration + teardown), in addition to the existing global fan-out. Additive to the existing requirements.

## Impact

- **Code:** orchestrator `events/` (sink-registration port, key-scoped subscription), `app/` (`Subscribe` command + lifecycle), `shared/bus.rs`/`boot.rs` (wiring); desktop `transport/` (`transport_subscribe!`, collapse `ChannelSink`/`SurfaceChannels` into the generic registry), `surface_host.rs` migration; `packages/client-bindings` (generic subscribe/stream helper); UI surface/terminal subscription call site.
- **Dependencies:** none new (tower already present; `ipc::Channel` already used).
- **ADRs:** new ADR for the subscribe/stream seam (subscribe-as-command, host-agnostic sink port, off-bus zero-copy stream, server-edge `Service`→`Stream`), relating ADR-0037 and ADR-0041.
- **Tests:** subscription register/teardown unit tests; key-scoped routing; surface-migration parity (byte path unchanged); macro expansion contract.
