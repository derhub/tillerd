PULL model (design.md). TDD per spec scenario (red → green); final task is a fix-all verify gate. Runtime is the post-infra-raw concrete `DaemonPtyApi` + `Runtime` enum in `infra/daemon_pty_api/` (no `SurfaceRuntime` trait).

## 1. Dispatch primitive (shared)

- [x] 1.1 Add `shared::bus::Broadcast<S: ?Sized>` (thread-safe `subscribe(Arc<S>)`, synchronous `dispatch(impl Fn(&S))`, no-subscriber no-op) next to `Bus`, with unit tests for fan-out order and the empty no-op. Note the borrowed-event sink convention in `shared::message`.

## 2. events/ contract module

- [x] 2.1 Add `pub(crate) mod events;` in `lib.rs` and `events/surface.rs`: `SurfaceEvent<'a> { Bytes(&[u8]), Status(&str), Exit(&str), Error(&str) }`, `trait SurfaceSink { fn emit(&self, surface: &str, event: &SurfaceEvent<'_>) }`, `impl SurfaceSink for Broadcast<dyn SurfaceSink>` (fan-out terminal), and the closure blanket impl (D8). `app/surface` re-exports `pub use crate::events::surface::{SurfaceEvent, SurfaceSink}`. Test: one borrowed event reaches N subscribers, no copy.
- [x] 2.2 Add the `**/events/**` ast-grep rule (sibling of `message-dto`: event field/variant payloads must be plain built-ins) + tests/fixtures; ships `error` (greenfield, immediately green).

## 3. infra runtime becomes a raw pull source

- [x] 3.1 `infra/daemon_pty_api/`: define `Output { Bytes(Vec<u8>), Status(String), Exit(String), Error(String) }` + `SurfaceOutput { surface: String, output: Output }` (owned primitives). `daemon.rs`: the per-proxy read loop, instead of calling `sink.on_*`, sends a `SurfaceOutput` on an internal `mpsc` owned by `DaemonPtyApi`; add `recv(&self) -> Option<SurfaceOutput>`. Delete the `SurfaceEventSink` trait, the `sink` field, and `DaemonPtyApi::new`'s sink arg. `FakeRuntime` gains `recv()` + a way for tests to enqueue outputs; `Runtime` enum delegates `recv()`.

## 4. app owns the pump

- [x] 4.1 `app/surface/stream.rs`: `SurfaceStream { runtime, fanout: Arc<Broadcast<dyn SurfaceSink>> }` with `run()` that loops `runtime.recv().await`, matches the owned `Output` into a borrowed `SurfaceEvent`, and `fanout.emit(&surface, &event)`. Delete `app/surface/events.rs` (`SurfaceEvents`) — replaced by `events/surface.rs`.
- [x] 4.2 `boot.rs`: build `Arc<Broadcast<dyn SurfaceSink>>`, `subscribe(cfg.sink)`, construct the runtime without a sink, `tokio::spawn(SurfaceStream{..}.run())`. `Config.sink` becomes `Arc<dyn SurfaceSink>`. Delete `SinkAdapter`.

## 5. host adopts a closure sink

- [x] 5.1 `apps/desktop/src-tauri/src/transport/sink.rs`: the host subscribes a closure (D8) `Arc<dyn SurfaceSink>` that matches `SurfaceEvent` over primitive `&str`/`&[u8]` — `Bytes` → copy into the per-surface `ipc::Channel` (IPC edge), `Status`/`Exit`/`Error` → `app.emit`. Drop the `SurfaceEvents` impl and any entity import. Keep the channel registry helpers.

## 6. Verify gate

- [x] 6.1 Fix-all: `sg scan` + `sg test` green (no infra/entity export regression, `**/events/**` rule green), `bun run verify` green, `bun run e2e` green; confirm PTY bytes, status, and exit still reach the renderer unchanged.
