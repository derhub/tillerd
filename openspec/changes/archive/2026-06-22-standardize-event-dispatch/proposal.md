## Why

The orchestrator emits daemon-to-host streams (PTY bytes, status, exit) through one bespoke output port, `SurfaceEvents` — a fixed 1:1 sink with hardcoded `on_bytes`/`on_status`/`on_exit`/`on_error` methods. Every future stream (service health, daemon lifecycle, supervision, task progress) would grow its own ad-hoc sink the same way, with no shared shape, no fan-out, and no place to insert cross-cutting concerns. This is an early architectural area carried forward, so the access shape must be standardized once: a single way producers expose stream data and a single place middleware can sit in front, without copying payloads on the hot path.

## What Changes

- Introduce a synchronous, zero-copy, borrow-and-forward **event-dispatch** standard: one composable per-domain sink trait whose method carries a **borrowed** event (`fn emit(&self, key: &str, event: &DomainEvent<'_>)`), so the subscriber decides whether to borrow, copy, or clone — the orchestrator never copies.
- A reusable `Broadcast` terminal that fans one borrowed event out to N subscribers (1:N), and a middleware pattern: a layer is just another impl of the same sink trait wrapping an `Arc<dyn DomainSink>`, so telemetry/filter/rate-limit compose **in front** of dispatch without changing any emit call site.
- The invariant that makes zero-copy sound: delivery is **synchronous on the emitter thread and consumed before `emit` returns**. A layer that must retain (buffer/async) owns at its own edge and is explicitly off the zero-copy fast path.
- **BREAKING (pre-v1, internal) — push → pull.** The infra runtime stops pushing to a sink: its `SurfaceEventSink` port and `boot`'s `SinkAdapter` are **deleted**. It becomes a dumb raw **source** — `recv() -> Option<SurfaceOutput>` hands the next decoded frame (owned primitives) to the caller. `app/surface::SurfaceStream` owns the dispatch: it pulls each frame, borrows its payload into a `SurfaceEvent<'_>`, and fans out 1:N through `Broadcast`. The host subscribes a `SurfaceSink` (a closure over primitives). This puts the loop, the translation, and the fan-out entirely in `app`, matching ADR-0038.
- **Output names no entity.** `SurfaceOutput` carries the surface id as a primitive `String` (`surface.as_str()`), so the runtime's output path names no `entities` type — the `events/` contract is plain built-ins end to end. (The runtime's control-op id type is settled by `infra-raw-app-owns-domain`; this change owns only the output push→pull conversion.)
- Records the decision as an ADR. The CQS `Bus` stays `execute`/`query` only; stream I/O is dispatch, not a bus message (this is the role the removed `Io` trait wrongly tried to fill).

## Capabilities

### New Capabilities

- `event-dispatch`: the synchronous zero-copy borrow-and-forward dispatch standard — the per-domain sink trait shape (borrowed event in the method), the `Broadcast` 1:N terminal, the middleware-by-wrapping composition, the synchronous-delivery invariant, and the primitive-only boundary rule for events leaving the crate.

### Modified Capabilities

- `surface-runtime`: the surface output stops being an infra push sink and becomes the first instance of `event-dispatch` — infra is a raw `recv()` source, `app/surface::SurfaceStream` is the pump that borrows each frame into a `SurfaceEvent<'_>` and fans out through `Broadcast`; the host subscribes a `SurfaceSink` closure over primitives.

## Impact

- `crates/orchestrator/src/shared/bus.rs` — add `Broadcast<S>` (generic fan-out) next to `Bus`; `shared/message.rs` — note the borrowed-event sink convention.
- `crates/orchestrator/src/events/surface.rs` (NEW `pub(crate) mod events;`) — `SurfaceEvent<'_>` enum + `SurfaceSink` + `impl … for Broadcast<dyn SurfaceSink>` + closure blanket impl; `app/surface` re-exports.
- `crates/orchestrator/src/app/surface/events.rs` — **deleted** (the interim `SurfaceEvents` push trait); replaced by `events/surface.rs`. `app/surface/stream.rs` (NEW) — `SurfaceStream` pump.
- `crates/orchestrator/src/infra/daemon_pty_api/{mod,daemon,fake}.rs` — delete the `SurfaceEventSink` port + `sink` field; the read loop sends owned `SurfaceOutput` on an internal `mpsc`; add `recv()`.
- `crates/orchestrator/src/boot.rs` — delete `SinkAdapter`; build `Broadcast`, subscribe `cfg.sink` (`Arc<dyn SurfaceSink>`), construct the sink-less runtime, spawn `SurfaceStream::run`.
- `apps/desktop/src-tauri/src/transport/sink.rs` — host subscribes a `SurfaceSink` closure over primitives, copying bytes into the per-surface `tauri::ipc::Channel` at the IPC edge.
- `.ast-grep/rules/**/events-*` — new `**/events/**` plain-built-ins rule + fixtures.
- `docs/adr/0037-*.md` — records the standard.
- No new runtime dependency; no change to the CQS `Bus` surface.
