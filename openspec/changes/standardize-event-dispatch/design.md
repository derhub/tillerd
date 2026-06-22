## Context

The orchestrator streams daemon-to-host output (PTY bytes, status, exit) in-process: the PTY proxy in `infra/runtime` decodes wire frames and **pushes** to a fixed sink (`SurfaceEventSink`, four methods), bridged to an app-facing `SurfaceEvents` by `SinkAdapter` in `boot.rs`. That puts the dispatch (the loop, the sink, the fan-out shape) inside infra, and every future outbound stream would clone it.

ADR-0038 settles the layering this standard must fit: **infra is a dumb raw API; `app/` owns all domain logic and dispatch.** So the standard makes the surface runtime a raw **source** (decode frames, hand them over on request) and moves the dispatch — the loop, the translation to events, the fan-out — entirely into `app/`. Constraints in force: raw bytes end-to-end (no stripping/re-decode); backpressure (ADR-0007); `entities`/`infra` internal, `app` the only public surface. The CQS `Bus` (`execute`/`query`) is request/result, not a stream vehicle (the removed `Io` trait).

## Goals / Non-Goals

**Goals:**

- The surface runtime (infra) is a raw source: control ops + `recv()` of decoded frames. It owns no sink and runs no dispatch loop, and never names `events/`.
- `app/` owns the dispatch: a pump pulls frames, borrows each payload into an `events::SurfaceEvent`, and fans out 1:N. Middleware composes by wrapping the sink trait.
- Zero copy: the payload is borrowed from the decoded frame through the fan-out to the subscriber; the subscriber alone chooses to borrow, copy, or clone.
- `events/` is the public transport contract (plain primitives), re-exported by `app`; the host knows only `app`.

**Non-Goals:**

- No async/queued/pub-sub event bus. Decoupled delivery forfeits zero-copy and is out of scope.
- Surface is the only domain now; the standard is shaped so adding one is an enum + a sink trait + a `Broadcast`.
- No change to the CQS `Bus`; no new runtime dependency.

## Decisions

### D1 — Synchronous borrow-and-forward, not a channel

Fan-out is a synchronous call passing a borrowed event, consumed before it returns. Zero-copy is only sound when the borrow cannot outlive the call. A channel (`mpsc`/broadcast) takes ownership (per-frame `Vec`) and queues — the opposite of a pass-through. *Note:* the pump pulls an already-owned decoded frame (the one inherent decode alloc) and borrows from it; it does not add a second channel.

### D2 — One trait is the composition point

A single per-domain trait with the borrow in the method (`fn emit(&self, key: &str, event: &DomainEvent<'_>)`) is implemented by the fan-out terminal, by middleware, and by closures. The borrow's lifetime lives in the method, so the trait stays `'static` and object-safe. *Rejected:* a generic `Emitter<E>` storing the event — a borrowed enum is a family of types, not a storable `'static` parameter.

### D3 — Generic core is only the fan-out

`Broadcast<S: ?Sized>` (thread-safe registration + synchronous iteration) is the one reusable type. Each domain supplies its own borrowed-enum event + sink trait. Keeps the generic surface tiny, no type-erasure on the hot path; the generic is justified by fan-out reuse + the middleware point, not a single impl.

### D4 — `events/` is an internal module; `app` is the only public surface

`crate::events/` is **`pub(crate)`**, fully internal like `entities/` and `infra/`. `app` re-exports the contract (`pub use crate::events::surface::{SurfaceEvent, SurfaceSink}`) just as it exposes commands/queries/`SurfaceView`. The host imports `orchestrator::app::…` only. `events/` types SHALL be **plain built-in Rust types** (`&str`, `&[u8]`, `String`, ints, `bool`, `Option`/`Vec` of those) — enforced by the `**/events/**` rule (D6). `events` differs from `entities` only in being primitives, so `app` may re-export it. The generic mechanism stays in `shared` (`Broadcast` in `shared/bus.rs`, the sink convention noted in `shared/message.rs`).

### D5 — infra is a raw pull source; `app` owns the pump (ADR-0038)

The surface runtime exposes `recv() -> Option<SurfaceOutput>` (the next decoded frame, owned primitives) plus control ops; it holds no sink, names no `events/` type, and runs no dispatch. `app`'s `SurfaceStream::run` is the dispatch: it pulls each frame, borrows its payload into an `events::SurfaceEvent`, and fans out through `Broadcast`. *Why pull, not a push port:* a push sink (infra calling a sink) puts the loop and the translation in infra; pull keeps infra a dumb source and the dispatch in `app`, matching ADR-0038. Zero-copy holds — the decoded frame is owned by the pump's stack each iteration, the event borrows it for the synchronous fan-out, and there is no channel between infra and app. (The runtime's port-vs-concrete shape is settled by the `infra-raw-app-owns-domain` change.)

### D6 — `**/events/**` ast-grep rule: plain built-in types only

A new rule (sibling of `message-dto`) over `**/events/**` flags any event field/variant payload whose type is not a plain built-in. It ships `error` once the tree is green (greenfield module → immediately). "Events are primitives" becomes a gate, not a convention.

### D7 — Backpressure is implicit

The pump's fan-out is synchronous, so a slow subscriber stalls `SurfaceStream::run`, which stops calling `recv()`, which lets the daemon socket buffer fill — natural backpressure, no unbounded queue, consistent with ADR-0007. A subscriber that must retain or hand off owns a copy at its own edge.

### D8 — Clients subscribe with a closure

A blanket impl makes any closure a sink, so a consumer needs no struct/impl — it subscribes with one closure capturing its state:

```rust
impl<F> SurfaceSink for F
where F: Fn(&str, &SurfaceEvent<'_>) + Send + Sync + 'static
{ fn emit(&self, surface: &str, event: &SurfaceEvent<'_>) { self(surface, event) } }
```

### Worked example — daemon PTY source to tauri client subscriber

**1. `shared/bus.rs` — generic fan-out (next to `Bus`)**

```rust
pub struct Broadcast<S: ?Sized> { subs: RwLock<Vec<Arc<S>>> }
impl<S: ?Sized> Default for Broadcast<S> { fn default() -> Self { Self { subs: RwLock::new(Vec::new()) } } }
impl<S: ?Sized> Broadcast<S> {
    pub fn subscribe(&self, sink: Arc<S>) { self.subs.write().unwrap_or_else(|e| e.into_inner()).push(sink); }
    pub fn dispatch(&self, f: impl Fn(&S)) {
        for s in self.subs.read().unwrap_or_else(|e| e.into_inner()).iter() { f(&**s); }
    }
}
```

**2. `events/surface.rs` — public contract, plain built-ins (`pub(crate)`, app re-exports)**

```rust
use crate::shared::bus::Broadcast;

pub enum SurfaceEvent<'a> { Bytes(&'a [u8]), Status(&'a str), Exit(&'a str), Error(&'a str) }

pub trait SurfaceSink: Send + Sync + 'static {
    fn emit(&self, surface: &str, event: &SurfaceEvent<'_>);     // &str, never an entity
}
impl SurfaceSink for Broadcast<dyn SurfaceSink> {                // fan-out terminal
    fn emit(&self, surface: &str, event: &SurfaceEvent<'_>) { self.dispatch(|s| s.emit(surface, event)); }
}
impl<F> SurfaceSink for F                                        // D8: closures are sinks
where F: Fn(&str, &SurfaceEvent<'_>) + Send + Sync + 'static
{ fn emit(&self, surface: &str, event: &SurfaceEvent<'_>) { self(surface, event) } }
```
`lib.rs`: `pub(crate) mod events;`. `app`: `pub use crate::events::surface::{SurfaceEvent, SurfaceSink};`

**3. Middleware — same trait, wraps inner**

```rust
pub struct LogStatus { pub inner: Arc<dyn SurfaceSink> }
impl SurfaceSink for LogStatus {
    fn emit(&self, surface: &str, event: &SurfaceEvent<'_>) {
        if let SurfaceEvent::Status(s) = event { tracing::debug!(surface, status = s, "surface status"); }
        self.inner.emit(surface, event);                         // forward same borrow
    }
}
```

**4. `infra/runtime` — DUMB raw source: control + `recv()`. No sink, no events, no dispatch.**

```rust
pub enum Output { Bytes(Vec<u8>), Status(String), Exit(String), Error(String) }   // owned by decode, primitives
pub struct SurfaceOutput { pub surface: String, pub output: Output }

impl SurfaceRuntime {                       // (port-or-concrete per infra-raw change)
    async fn spawn(/* req */) -> Result<()>; async fn input(..); async fn resize(..); async fn stop(..);
    async fn recv(&self) -> Option<SurfaceOutput>;   // next decoded frame for any surface
}
```

**5. `app/surface` — the pump IS the dispatch (loop + translate + fan out)**

```rust
use crate::events::surface::{SurfaceEvent, SurfaceSink};
use crate::shared::bus::Broadcast;

pub struct SurfaceStream {
    runtime: Arc<dyn crate::infra::runtime::SurfaceRuntime>,
    fanout:  Arc<Broadcast<dyn SurfaceSink>>,
}
impl SurfaceStream {
    pub async fn run(&self) {
        while let Some(SurfaceOutput { surface, output }) = self.runtime.recv().await {
            let event = match &output {                       // borrow the owned frame — zero extra copy
                Output::Bytes(b)  => SurfaceEvent::Bytes(b),
                Output::Status(s) => SurfaceEvent::Status(s),
                Output::Exit(q)   => SurfaceEvent::Exit(q),
                Output::Error(r)  => SurfaceEvent::Error(r),
            };
            self.fanout.emit(&surface, &event);               // synchronous fan-out
        }
    }
}
```

**6. `boot.rs` — build the fan-out, subscribe the host, spawn the pump**

```rust
let fanout: Arc<Broadcast<dyn SurfaceSink>> = Arc::default();
fanout.subscribe(cfg.sink);                                   // host closure (or wrap in LogStatus)
tokio::spawn(SurfaceStream { runtime, fanout }.run());
```

**7. host — subscribes a closure (D8); imports only `app`**

```rust
use orchestrator::app::surface::{SurfaceEvent, SurfaceSink};

let (channels, app) = (self.channels.clone(), self.app.clone());
cfg.sink = Arc::new(move |surface: &str, event: &SurfaceEvent<'_>| match event {
    SurfaceEvent::Bytes(b)  => deliver(&channels, surface, b),                  // borrow -> ipc channel
    SurfaceEvent::Status(s) => { let _ = app.emit(STATUS_EVENT, json!({"surfaceId": surface, "status": s})); }
    SurfaceEvent::Exit(q)   => { let _ = app.emit(EXIT_EVENT, json!({"surfaceId": surface, "qualifier": q})); /* drop channel */ }
    SurfaceEvent::Error(r)  => { let _ = app.emit(ERROR_EVENT, json!({"surfaceId": surface, "reason": r})); }
});
```

Flow: PTY frame → infra decodes (owns the `Vec`) → app `SurfaceStream::run` pulls it via `recv()` → borrows the payload into `events::SurfaceEvent` → optional middleware → `Broadcast` fan-out → host closure. Infra is a source only; the dispatch loop, translation, and fan-out are all in `app`. `events::SurfaceEvent` is primitives; `entities::SurfaceId` never leaves the crate; the host names only `app`. The tauri `ipc::Channel<Vec<u8>>` needs owned bytes to cross the process boundary, so the client copies the `&[u8]` **at its own edge** — its choice, forced by IPC. Infra, the pump, and every middleware stayed zero-copy.

### Performance

Per output frame the new shape adds, over the decode that already happens: a stack `SurfaceEvent` (fat pointer + tag, no heap), an enum match, and one `RwLock::read` + N virtual calls for the fan-out. **No new allocation, no new copy.** Negligible: PTY output is chunked (KB frames); each frame already costs an async socket read/ack (a syscall) that dwarfs a lock-read + indirection; the borrowed enum guarantees the payload is never copied in-process. The one unavoidable copy is the host IPC boundary, which exists today.

## Risks / Trade-offs

- **A blocking subscriber stalls the pump** → subscribers must be fast or own-and-hand-off at their edge. This is also the backpressure mechanism (D7) — a feature.
- **`RwLock` read on the hot path under extreme fan-out** → drop-in swap to `ArcSwap<Arc<[Arc<S>]>>` for lock-free reads; subscribe/unsubscribe is rare.
- **Generic `Broadcast` risks looking ornamental** → justified by fan-out reuse + the middleware point; surface is the first of several anticipated domains.
- **infra `SurfaceOutput`/`Output` frame type vs `events::SurfaceEvent`** → not duplication: the frame is the raw decoded transport value (owned primitives, infra-owned); the event is the borrowed public contract built by the pump. Opposite ends of the boundary; the pump's match is the translation.

## Migration Plan

1. Add `shared::bus::Broadcast<S>`; note the sink convention in `shared::message`.
2. Add `events/` (`pub(crate) mod events;`): `SurfaceEvent` + `SurfaceSink` + `impl … for Broadcast<…>` + the closure blanket impl; `app` re-exports.
3. Add the `**/events/**` ast-grep rule + fixtures.
4. Turn the surface runtime into a raw source: expose `recv() -> Option<SurfaceOutput>` (owned primitive frame) and keep control ops; delete the `SurfaceEventSink` push sink and `SinkAdapter`. (Coordinated with `infra-raw-app-owns-domain` / the daemon-pty change.)
5. Add `app/surface::SurfaceStream` (the pump) and spawn it from `boot`; `Config.sink` becomes `Arc<dyn events::SurfaceSink>`.
6. Migrate the host `transport/sink.rs` to a closure sink over primitives.
7. `ast-grep scan` + `ast-grep test` green, `bun run verify` + `bun run e2e` green; bytes/status/exit reach the renderer unchanged.

Rollback: revert; no persisted data or wire-format depends on it.

## Open Questions

- Which additional domains adopt the standard next (service-health, daemon lifecycle, supervision, task progress), and do any need >1 subscriber.
- A tiny generic envelope (`topic()`/id) for cross-domain observing middleware now, or deferred until a second domain exists.
- Implementation order is `infra-raw-app-owns-domain` first (domain-logic moves + daemon kind-agnostic; surface output streaming left unchanged), then this change. This change owns the push→pull conversion in full: the `recv()` raw-source API, `events/`, the pump, closure clients, and deleting the old `SurfaceEventSink`/`SinkAdapter`. It builds on the already-raw, kind-agnostic daemon, so there is no overlap with the first change.
