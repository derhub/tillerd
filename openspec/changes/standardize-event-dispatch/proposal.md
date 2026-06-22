## Why

The orchestrator emits daemon-to-host streams (PTY bytes, status, exit) through one bespoke output port, `SurfaceEvents` — a fixed 1:1 sink with hardcoded `on_bytes`/`on_status`/`on_exit`/`on_error` methods. Every future stream (service health, daemon lifecycle, supervision, task progress) would grow its own ad-hoc sink the same way, with no shared shape, no fan-out, and no place to insert cross-cutting concerns. This is an early architectural area carried forward, so the access shape must be standardized once: a single way producers expose stream data and a single place middleware can sit in front, without copying payloads on the hot path.

## What Changes

- Introduce a synchronous, zero-copy, borrow-and-forward **event-dispatch** standard: one composable per-domain sink trait whose method carries a **borrowed** event (`fn emit(&self, key: &str, event: &DomainEvent<'_>)`), so the subscriber decides whether to borrow, copy, or clone — the orchestrator never copies.
- A reusable `Broadcast` terminal that fans one borrowed event out to N subscribers (1:N), and a middleware pattern: a layer is just another impl of the same sink trait wrapping an `Arc<dyn DomainSink>`, so telemetry/filter/rate-limit compose **in front** of dispatch without changing any emit call site.
- The invariant that makes zero-copy sound: delivery is **synchronous on the emitter thread and consumed before `emit` returns**. A layer that must retain (buffer/async) owns at its own edge and is explicitly off the zero-copy fast path.
- **BREAKING (pre-v1, internal):** the surface output port `SurfaceEvents` is replaced by a `SurfaceSink` that takes a borrowed `SurfaceEvent<'_>` enum; the daemon emits through a chain head (`Arc<dyn SurfaceSink>` = `Broadcast` today) instead of a single fixed sink. The host transport implements the new trait.
- **Runtime goes fully wire-only (finishes the infra-raw de-domaining):** `infra/daemon_pty_api` stops naming `entities::SurfaceId` — its call API, the `proxies` key, and the primitive output sink all speak `contracts::SessionId` (the daemon's wire id; `SessionId(surface.as_str())` is a lossless derivation). The `boot` `Bridge` owns the `SessionId ↔ SurfaceId` translation: app passes `SessionId` into the runtime and maps the emitted wire id back to `SurfaceId` when constructing `SurfaceEvent`. This is deferred here (not into `infra-raw-app-owns-domain`) because the only lever forcing `SurfaceId` into infra is the sink param type, which this change already rewrites — doing it there would need an interim reverse registry.
- Records the decision as an ADR. The CQS `Bus` stays `execute`/`query` only; stream I/O is dispatch, not a bus message (this is the role the removed `Io` trait wrongly tried to fill).

## Capabilities

### New Capabilities

- `event-dispatch`: the synchronous zero-copy borrow-and-forward dispatch standard — the per-domain sink trait shape (borrowed event in the method), the `Broadcast` 1:N terminal, the middleware-by-wrapping composition, the synchronous-delivery invariant, and the primitive-only boundary rule for events leaving the crate.

### Modified Capabilities

- `surface-runtime`: the surface output port stops being a fixed `SurfaceEvents` sink and becomes the first instance of `event-dispatch` — a `SurfaceSink` consuming a borrowed `SurfaceEvent<'_>`, emitted through a `Broadcast` chain head. The runtime (`infra/daemon_pty_api`) becomes fully wire-only: it speaks `contracts::SessionId`, names no `entities` type, and the `Bridge` translates wire id ↔ `SurfaceId` at the app boundary.

## Impact

- `crates/orchestrator/src/shared/bus.rs` — add `Broadcast<S>` (generic fan-out) next to `Bus`.
- `crates/orchestrator/src/shared/message.rs` — add the generic borrowed-event sink contract/convention next to `Command`/`Query`.
- `crates/orchestrator/src/app/surface/events.rs` — `SurfaceEvents` → `SurfaceEvent<'_>` enum + `SurfaceSink` + `impl SurfaceSink for Broadcast<dyn SurfaceSink>` (host-facing, like `SurfaceView`).
- `crates/orchestrator/src/infra/daemon_pty_api/mod.rs`, `daemon.rs` (renamed from `infra/runtime/` by `infra-raw-app-owns-domain`) — the primitive `SurfaceEventSink` port and the read loop stay in shape, but their id type and the call API/`proxies` key move from `entities::SurfaceId` to `contracts::SessionId` so infra names no domain type.
- `crates/orchestrator/src/boot.rs` — `SinkAdapter` becomes the `Bridge` that constructs `SurfaceEvent` from the infra port (mapping the emitted `SessionId` back to `SurfaceId`) and fans out through `Broadcast`; subscribe the host sink. App call sites pass `SessionId` into the runtime.
- `apps/desktop/src-tauri/src/transport/sink.rs` — host implements `SurfaceSink` over its per-surface `tauri::ipc::Channel`; primitives only.
- `docs/adr/0037-*.md` — records the standard.
- No new runtime dependency; no change to the CQS `Bus` surface.
