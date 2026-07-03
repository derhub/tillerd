## Context

Client-provided streaming exists today only as a bespoke one-off for surface output: `surface_create` is a hand-written `#[tauri::command]` (because `tauri::ipc::Channel` cannot live in the core bus), `ChannelSink` holds a Tauri-specific `HashMap<surface_id, ipc::Channel<Vec<u8>>>`, and `makeSurfaceChannel` is a bespoke client binding. The orchestrator already has the right primitives — `Broadcast<dyn SurfaceSink>` (global fan-out), the borrowed zero-copy `SurfaceEvent<'a>` standard (ADR-0037), and the `SurfaceStream` pump — but no reusable `subscribe`/`stream` verb, no host-agnostic sink port, and no transport macro.

This change generalizes that bespoke path into a reusable `stream-subscription` capability and migrates surface output onto it. The bus middleware spine (ADR-0041) and the event-dispatch standard (ADR-0037) are the foundation.

## Goals / Non-Goals

**Goals:**

- A host-agnostic, key-scoped sink-registration mechanism a client subscribes to with its own sink.
- `subscribe` as a tower-observed bus command; frames stream zero-copy through the registered sink, never re-dispatched per frame.
- A `transport_subscribe!` macro so streaming endpoints are declared, not hand-written.
- A generic client subscribe/stream binding generalizing `makeSurfaceChannel`.
- Migrate surface output onto the mechanism, preserving its byte path and (where possible) its wire shape.

**Non-Goals:**

- New stream endpoints (logs follow, task progress) — follow-on; logs follow uses `notify` direct + the non-blocking-I/O sweep (see proposal).
- The server/web host adapter implementation (the port enables it; building it is later).
- Per-frame middleware as a shipped feature (the sink-wrapping tier is available; not built now).
- Forcing UI/Phase-1 changes — the surface migration keeps `surface_create`'s wire shape so the renderer is unaffected; the generic client binding lands additively.

## Decisions

### D1. Subscribe = bus command carrying the client sink; frames bypass per-frame dispatch

A subscription is a `Command` carrying an `Arc<dyn SurfaceSink>` (the client sink adapter). `bus.execute(SubscribeSurface { key, sink })` runs the full tower pipeline once (middleware observes the subscribe), and the handler registers the sink in a key-scoped registry. After that, the `SurfaceStream` pump delivers each borrowed frame straight to the registered sinks — no `bus.execute` per frame, no owned `Op<T>` future per frame, no copy by the core. The single owned copy is the sink's `to_vec()` at the IPC edge (structural; ADR-0037-sanctioned; unavoidable when leaving the process).

- **Why:** keeps middleware on subscription setup (logging/auth per subscription) while the high-rate byte path stays zero-copy. Routing frames through the owned bus dispatch would force a copy + pipeline pass per frame.

### D2. Host-agnostic, key-scoped sink registry

Generalize the global `Broadcast<dyn SurfaceSink>` into a **key-scoped registry**: register/remove `Arc<dyn Sink>` under a key (surface id), deliver a borrowed event only to that key's sinks. The core speaks the sink trait only — never a `tauri::ipc::Channel`. The desktop adapter (`ChannelSink`) wraps an `ipc::Channel`; a future server adapter wraps a WebSocket. This collapses today's `SurfaceChannels` `HashMap` + `ChannelSink` routing into one reusable mechanism.

- Registration/teardown safe concurrently with delivery (the existing `RwLock` over the subscriber list, now keyed).

### D3. `transport_subscribe!` macro

A new transport macro mirrors `transport_command!`: it takes the client `ipc::Channel` + params, builds the host adapter sink, and dispatches the subscribe command. So a streaming endpoint is one macro line, not a hand-written command.

### D4. Generic client binding

Generalize `makeSurfaceChannel` into a typed subscribe/stream helper in `@tillerd/client-bindings` (create a typed channel, call the subscribe command, expose teardown). Lands additively; `makeSurfaceChannel` can become a thin alias.

### D5. Migrate surface output, wire-compatible

`surface_create`/attach/detach move to the `transport_subscribe!` form over the key-scoped registry; `ChannelSink`/`SurfaceChannels` collapse into the generic registry. Keep `surface_create`'s invoke wire shape so the renderer (Phase-1-broken) needs no change; the generic client binding is additive.

### D6. The edge copy stays

The one `to_vec()` at the sink→`ipc::Channel` boundary is structural (data leaving the process), cheap (one memcpy before a mandatory IPC hop), and accepted. The valuable zero-copy (internal fan-out across N sinks) is preserved.

## Risks / Trade-offs

- **Migrating working surface code** -> regression risk on the terminal byte path. Mitigation: keep the wire shape; parity tests (a frame for a subscribed surface reaches its sink; teardown stops it); backend-verifiable without the UI.
- **Key-scoped registry vs the existing global `Broadcast`** -> behavior change for surface routing (today `ChannelSink` routes by id inside one global sink). Mitigation: the registry encodes the same routing; the global-Broadcast surface path is replaced wholesale, not run in parallel.
- **Stacking on unpushed `tower-bus-middleware`** -> this change builds on committed-but-unpushed work on the same branch. Mitigation: backend stays green; push ordering resolved later (with Phase 1).
- **Closed client sink** -> a dropped `ipc::Channel` send must not block the pump. Mitigation: send is non-blocking + the teardown/closed-sink scenarios are specced and tested.

## Migration Plan

1. Key-scoped sink registry in `events/` (generalize the surface fan-out); registration/teardown.
2. `SubscribeSurface` command + handler (register sink under surface id).
3. `transport_subscribe!` macro; desktop `ChannelSink` becomes the host adapter over the generic registry; collapse `SurfaceChannels`.
4. Migrate `surface_create`/attach/detach onto the macro, wire-compatible.
5. Generic client binding; `makeSurfaceChannel` -> thin alias (additive, no UI change required).
6. Parity + teardown + key-scoping tests; backend gate (`cargo test -p tillerd-orchestrator -p tillerd-desktop`, clippy, sg).

Rollback: pre-v1 internal seam; revert restores the bespoke surface path.

## Open Questions

- Whether the key-scoped registry replaces `Broadcast<dyn SurfaceSink>` outright or wraps it — resolved in APPLY task 1 against the existing `boot.rs` fan-out wiring; does not change the spec contract.
- Exact teardown trigger on the desktop (channel drop vs explicit unsubscribe command) — APPLY task 3; both satisfy the teardown scenarios.
