TDD per spec scenario (red → green); the final task is a fix-all verify gate.

## 1. Dispatch standard (shared)

- [ ] 1.1 Add `shared::bus::Broadcast<S: ?Sized>` (thread-safe `subscribe`, synchronous `dispatch`) next to `Bus` + unit tests for fan-out order and the no-subscriber no-op. Add the generic borrowed-event sink contract/convention to `shared::message`.
- [ ] 1.2 In `app/surface/events.rs`, replace `SurfaceEvents` with `SurfaceEvent<'a>` + `SurfaceSink` + `impl SurfaceSink for Broadcast<dyn SurfaceSink>`. Test: one borrowed event reaches N subscribers with no copy.

## 2. Surface adopts the standard

- [ ] 2.1 Leave the infra `SurfaceEventSink` port and the `daemon.rs` read loop unchanged (still emits primitives by `SurfaceId`).
- [ ] 2.2 Rewire `boot.rs`: build the `Broadcast`, subscribe `cfg.sink`, and pass a `Bridge` (impl of infra `SurfaceEventSink` that wraps each borrowed callback into `SurfaceEvent` and fans out) to the runtime. `Config.sink` becomes `Arc<dyn SurfaceSink>`; `SinkAdapter` becomes `Bridge`.
- [ ] 2.3 Migrate host `apps/desktop/src-tauri/src/transport/sink.rs` to `impl SurfaceSink for ChannelSink` over primitives (match on `SurfaceEvent`, copy bytes into the ipc channel at the IPC edge); drop the entity `SurfaceId` import.

## 3. Verify gate

- [ ] 3.1 Fix-all: `ast-grep scan` + `ast-grep test` green (no infra/entity export regression), `bun run verify`, then `bun run e2e` green; confirm PTY output, status, and exit still reach the renderer unchanged.
