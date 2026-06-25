# 0042. Client-provided stream subscriptions: subscribe-as-command, host-agnostic sink port, zero-copy passthrough

- Status: proposed
- Date: 2026-06-25
- Relates: ADR-0037 (zero-copy event dispatch), ADR-0041 (tower bus middleware spine)

## Context

Client-provided streaming exists only as a bespoke one-off for surface output: `surface_create` is hand-written because `tauri::ipc::Channel` cannot live in the core bus, `ChannelSink` is Tauri-specific, and `makeSurfaceChannel` is a bespoke binding. There is no reusable `subscribe`/`stream` verb, no host-agnostic sink port, and the expected web/server host cannot serve streams. The orchestrator already has the primitives — `Broadcast`, the borrowed zero-copy `SurfaceEvent<'a>` standard (ADR-0037), the `SurfaceStream` pump — they are just not generalized.

A design study (web-researched) compared the tower-standard streaming shape (`Service` whose `Response` is a `Stream`, as in tonic/axum SSE) against the bespoke sink path. The tower-standard shape is network-transport-shaped: it requires owned `Stream` items (a copy per frame, breaking ADR-0037 zero-copy) and a framework that drives the stream to a wire — and Tauri commands cannot return a stream at all. So it does not fit the in-process desktop core; it fits only a network host's own edge.

## Decision

Client-provided streams use a subscribe-as-command model with a host-agnostic sink port.

- **`subscribe` is a tower-observed bus command.** Establishing a subscription is a `Command` carrying a client-provided `Arc<dyn Sink>`. It runs through the full middleware pipeline once (logging, future auth), registers the sink, and returns. Subscription setup — not each frame — is what middleware observes.

- **Frames stream zero-copy through the registered sink.** After registration the source pump delivers each borrowed frame straight to the registered sinks. Frames are NOT re-dispatched through `bus.execute` per frame (that would force an owned `Op<T>` future + a copy + a pipeline pass on a high-rate stream). The bus owns the subscription; the client `tx` is the frame channel.

- **Host-agnostic, key-scoped sink port.** The core registers/removes `Arc<dyn Sink>` under a key (surface/session id) and delivers a borrowed event only to that key's sinks. The core names no host transport type. The desktop adapter wraps `tauri::ipc::Channel`; a future server adapter wraps a WebSocket. The existing per-surface `ChannelSink` + `SurfaceChannels` map collapse into this one mechanism.

- **The one edge copy is structural and accepted.** A sink that hands data to its transport owns it at that point (`to_vec()` into `ipc::Channel`) — the single copy when data leaves the process. Internal fan-out across N sinks stays borrowed/zero-copy; only a subscriber that actually crosses the process boundary pays one copy, at its own edge. True zero-copy across the sandboxed webview boundary would need shared memory, which the host IPC does not expose; the copy is cheap (one memcpy before a mandatory IPC hop).

- **Two middleware tiers.** Subscribe setup -> tower Layer (the ADR-0041 pipeline). Per-frame, if ever needed -> ADR-0037 sink-wrapping (borrowed, synchronous, zero-copy), NOT the tower bus. Per-frame middleware is not built now.

- **`command`/`query` are unchanged** — they return values (Tauri Promise / HTTP body). A client `tx` is for stream arity + backpressure, not for unary calls and not a non-blocking mechanism. The verb/arity split (0/1/N values) maps cleanly to both desktop and a future server host.

- **Declared, not hand-written.** A `transport_subscribe!` macro defines a streaming endpoint like `transport_command!`; a generic client binding generalizes `makeSurfaceChannel`. Surface output migrates onto the mechanism, wire-compatible.

## Consequences

- One reusable streaming mechanism; a new stream is a key + a sink trait + a macro line, on any host.
- The bus stays request/response (ADR-0041); streams are dispatch (ADR-0037). Subscription setup is on the bus and observable; frames are not per-frame bus messages. This refines, not contradicts, both ADRs.
- The web/server host becomes able to serve streams: its adapter supplies a WebSocket/SSE sink to the same port, and may expose a tower `Service`->`Stream` at its own edge, fed by the port.
- A migration of working surface-streaming code, with regression risk bounded by keeping the wire shape and adding parity/teardown tests.
- One owned copy per subscriber at the process boundary remains; it is structural and cheap, and the meaningful zero-copy (internal multi-sink fan-out) is preserved.
