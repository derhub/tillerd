## 1. Key-scoped sink registry (specs: host-agnostic sink, key-scoped delivery, teardown) (D2)

- [x] 1.1 (red) Tests: register two sinks under keys A/B; an event for A reaches only A's sink; remove A's sink and an event for A reaches neither; register/remove concurrent with an in-flight dispatch is safe. (event-dispatch delta scenarios)
- [x] 1.2 (green) Add a key-scoped registry in `events/` generalizing the surface fan-out: `register(key, Arc<dyn Sink>)` / `remove(key, ..)` / borrowed key-scoped `dispatch`. Decide replace-vs-wrap of `Broadcast<dyn SurfaceSink>` against `boot.rs` wiring; keep borrowed/synchronous/zero-copy delivery.

## 2. Subscribe command + zero-copy passthrough (specs: subscription via bus, frames no per-frame dispatch) (D1)

- [x] 2.1 (red) Tests: a `SubscribeSurface` dispatched through the bus is observed by an installed middleware layer exactly once; subscribe returns after registration before any frame; a frame for a subscribed key reaches the registered sink without any per-frame command dispatch.
- [x] 2.2 (green) Add `SubscribeSurface { surface_id, sink: Arc<dyn SurfaceSink> }` `Command<Ctx>`; handler registers the sink in the key-scoped registry. Wire the `SurfaceStream` pump to deliver per-key through the registry. No `bus.execute` per frame.

## 3. Transport macro + desktop adapter migration (specs: host-agnostic sink) (D3, D5)

- [x] 3.1 (green) Add `transport_subscribe!` (takes the client `ipc::Channel` + params, builds the host `ChannelSink` adapter, dispatches the subscribe command).
- [x] 3.2 (green) Refactor `ChannelSink` into the host adapter over the generic registry; collapse the `SurfaceChannels` map. Migrate `surface_create`/attach/detach onto `transport_subscribe!`, keeping the invoke wire shape (no renderer change required). Resolve teardown trigger (channel drop vs explicit unsubscribe) to satisfy the teardown scenarios.

## 4. Client binding (additive, no UI change) (D4)

- [x] 4.1 (green) Generalize `makeSurfaceChannel` into a typed subscribe/stream helper in `@tillerd/client-bindings` (typed channel + subscribe call + teardown); keep `makeSurfaceChannel` as a thin alias. Additive — the Phase-1 renderer is unaffected.

## 5. Verify + fix-all gate

- [x] 5.1 Backend gate green: `cargo test -p tillerd-orchestrator -p tillerd-desktop` = 531 pass (parity: frame reaches subscribed sink; teardown stops it; key-scoping; register/remove during dispatch safe), `cargo clippy --workspace --all-targets -- -D warnings` clean, `sg scan` no new findings (only pre-existing client-bindings TS comment). Spec scenarios map 1:1 to tests except "closed client sink does not block source" (structural — `ipc::Channel.send` is fire-and-forget; no unit test). Full `bun run verify`/e2e deferred (pre-existing broken UI; backend + additive binding only).
- [x] 5.2 Confirmed `docs/adr/0042-client-provided-stream-subscriptions.md` matches the shipped shape (subscribe-as-command, host-agnostic key-scoped registry, zero-copy passthrough, single delivery, structural edge copy).
