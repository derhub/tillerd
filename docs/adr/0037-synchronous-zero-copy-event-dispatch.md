# 0037. Synchronous zero-copy event dispatch: borrowed events, fan-out terminal, middleware by wrapping

- Status: accepted (bus-exclusivity clause revised by ADR-0041)
- Date: 2026-06-22

## Context

The orchestrator streams daemon-to-host output (PTY bytes, status, exit) through the orchestrator
process: the PTY proxy in `infra/runtime/daemon.rs` decodes wire frames and calls a fixed output port,
`SurfaceEventSink` (infra), bridged to the app-facing `SurfaceEvents` by a `SinkAdapter` in `boot.rs`.
That port is a hardcoded 1:1 trait with four methods and no place for cross-cutting concerns. Every
future outbound stream (service health, daemon lifecycle, supervision, task progress) would clone the
same ad-hoc shape.

This is an early, load-bearing area, so the access shape is standardized once. The CQS `Bus`
(`execute`/`query`) is request/result and is not the vehicle for streams; the short-lived `Io` message
trait that tried to model I/O on the bus was a structural duplicate of `Query` with no implementors and
is removed. Constraints in force: raw bytes end-to-end (no stripping/re-decode), the
reliability/operability contract including backpressure (ADR-0007), and entities/infra staying internal
to the crate (`entities-stay-internal`, `infra-no-export`).

## Decision

Daemon-to-host streams use a synchronous, zero-copy, borrow-and-forward dispatch standard.

- **Borrowed events, synchronous delivery.** A domain exposes a sink trait whose method takes a
  **borrowed** event (`fn emit(&self, key: &str, event: &DomainEvent<'_>)`). Delivery is synchronous on
  the producer's thread and the borrow is consumed before `emit` returns. The producer never copies;
  the subscriber alone chooses to borrow, copy, or clone. This is the only model under which an
  in-memory event system is zero-copy — a channel (`mpsc`/broadcast) would take ownership (per-frame
  allocation) and queue, which is a buffer, not a pass-through.

- **One trait is the composition point.** The same per-domain sink trait is implemented by the fan-out
  terminal, by middleware, and by the chain head the producer holds. The borrow lives in the method, so
  the trait is `'static` and object-safe. A generic `Emitter<E>` storing the event type is rejected: a
  borrowed enum `Event<'a>` is a family of types and cannot be a stored `'static` parameter.

- **The generic core is only the fan-out.** One reusable type, `Broadcast<S: ?Sized>` (thread-safe
  registration + synchronous iteration), delivers one borrowed event to N subscribers in registration
  order. Each domain supplies its own borrowed-enum event and sink trait. No type-erasure or downcast on
  the hot path.

- **Middleware composes by wrapping.** A layer (telemetry, filter, rate-limit) implements the sink trait
  and wraps an inner `Arc<dyn DomainSink>`. Inserting or removing a layer never changes a producer emit
  call site. An observing/forwarding layer preserves the borrow and adds no copy. Cross-domain middleware
  operates on the key/envelope; payload-aware middleware is per-domain.

- **Generic in `shared`, typed event in `events/`.** The domain-free pieces live next to their CQS
  siblings: `Broadcast<S>` (fan-out transport) in `shared/bus.rs`, the borrowed-event sink convention in
  `shared/message.rs`. The typed `SurfaceEvent<'a>` + `SurfaceSink` are the host-facing transport
  contract (plain built-ins), so they live in a `pub(crate) mod events` (`events/surface.rs`), re-exported
  by `app` exactly as `SurfaceView` is. `events/` is internal like `entities/`/`infra/` but holds only
  primitives, enforced by the `**/events/**` rule. The host imports `orchestrator::app::…` only.

- **Surface output is the first instance — a pull source, app owns the dispatch.** The infra runtime is a
  dumb raw source: its proxy read loop puts each decoded frame (owned `SurfaceOutput`, primitives) on an
  internal queue, exposed by `recv()`. It holds no sink and names no `events/` type. `app/surface::SurfaceStream`
  is the pump: it pulls each frame, borrows the payload into a `SurfaceEvent<'_>`, and fans out through
  `Broadcast<dyn SurfaceSink>`. The old infra `SurfaceEventSink` push port and `boot`'s `SinkAdapter` are
  deleted; the host subscribes a `SurfaceSink` closure over primitives. Pull (not a push port) keeps the
  loop and the translation in `app`, matching ADR-0038; zero-copy holds because the owned frame lives on
  the pump's stack and the event borrows it for the synchronous fan-out, with no channel between infra and `app`.

- **Backpressure is implicit.** Synchronous delivery means a slow subscriber slows the read loop, which
  slows draining the PTY socket — bounded by construction, no unbounded buffer, consistent with ADR-0007.
  A subscriber that must retain or hand off to another thread owns a copy at its own edge and is then
  off the zero-copy path; the standard itself stores nothing.

## Consequences

- One standard for all daemon-to-host streams; a new domain is a borrowed-enum + a sink trait + a
  `Broadcast`. Fan-out (1:N) and a middleware insertion point come for free.
- Zero copy in-process: PTY bytes are borrowed from the decoded frame to the subscriber; the only copy is
  the host's IPC boundary (e.g. tauri `ipc::Channel<Vec<u8>>`), which exists today and is the client's
  choice at its edge.
- No measurable overhead: bytes already pass through the orchestrator; the new shape adds a stack enum, a
  match, and (for fan-out) a lock read plus N indirections — dwarfed by the per-frame socket `ack`
  already on the path, and amortized over KB-scale chunks. `RwLock` reads can become lock-free
  (`ArcSwap`) if ever measured.
- A subscriber must not block the producer thread; the synchronous invariant is documented on the trait.
- The CQS `Bus` stays `execute`/`query` only; streams are dispatch, not bus messages.
