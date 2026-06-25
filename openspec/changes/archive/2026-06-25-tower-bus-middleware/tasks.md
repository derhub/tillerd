## 1. Prep — collapse Ctx (D0)

- [x] 1.1 Change `Ctx` to `Ctx(Arc<CtxInner>)` with `CtxInner { db, kv, fs_root, runtime }`; forward all accessors (`db`/`kv`/`fs_root`/`runtime`/`runtime_arc`) and `transaction` through the inner. Existing `Ctx`/`boot` tests stay green (no test changes); per-dispatch clone is now one atomic.

## 2. tower wrapper + error-logging layer (specs: dispatch composes layers; error-logging) (D1, D2)

- [x] 2.1 Add `tower` to `crates/orchestrator/Cargo.toml` (minimal features: `Layer`/`Service`/`util`).
- [x] 2.2 (red) Test: a no-op observing layer installed around dispatch is invoked for a command, the handler runs unchanged, and the result is unaffected; a second test asserts two layers run in composition order. (spec: "A layer observes a dispatched command…", "Layer order follows composition order")
- [x] 2.3 (green) Add the erased `OpEnvelope` (action name, kind, optional `&dyn Notable`, boxed handler future) as the Service `Request`; add `HandlerService` adapter that owns a `Ctx` clone and runs the handler; wire `execute`/`query` to drive `ServiceBuilder`-composed layers via `oneshot`. Keep `execute<C>`/`query<Q>` signatures. Handlers stay plain `Command`/`Query`.
- [x] 2.4 (red→green) Move the per-call span + `inspect_err(record)` into an `ErrorLoggingLayer`; the existing tests `a_command_error_logs_exactly_one_error_event_with_the_stable_code` and `a_successful_operation_logs_no_error_event` MUST pass unchanged (same OTel field shape, one event per failure, none on success).
- [x] 2.5 (red→green) Test: surface input/resize/attach traffic invokes no installed layer (raw runtime I/O stays off the layered path). (spec: "Surface input bytes never reach a bus layer")

## 3. Lifecycle signals on the bus + notification-recording layer (specs: signals observable; single recording point) (D3)

- [x] 3.1 (red) Test: a `Notable` bus operation dispatched through the bus is observed by an installed layer that reads its `notification()`. (spec: "A surface start is observable…", "An orchestrator status change is observable…")
- [x] 3.2 (green) Add `trait Notable { fn notification(&self) -> Option<NotificationWire>; }`; expose `&dyn Notable` on `OpEnvelope`; define the `surface_started` and `orchestrator_status` signals as `Notable` bus operations carrying primitive data.
- [x] 3.3 (red→green) Add `NotificationRecordingLayer` that records exactly one notification per observed `Notable` signal; test single-recording (one signal → one notification; no second recorder records the same signal). (spec: "An observed surface start becomes one recorded notification", "A status change is recorded once")
- [x] 3.4 (green) Route the producers onto the bus: `surface_host` surface-start path and `orchestrator_host` notification-worthy status dispatch the `Notable` signal through the bus instead of the desktop recorder / notification `app.emit`. Keep boot-phase `orchestrator://status` progress emits (UI, not notifications).

## 4. Compose at boot + retire the desktop recorder (D2, migration)

- [x] 4.1 (green) Compose the stack in `boot.rs::build_bus`: error-logging → notification-recording → handler; hold it in `Bus`.
- [x] 4.2 Remove the desktop mpsc recorder and `spawn_recorder`/`NotificationRecorder` notification path (b980eac) and the off-bus notification emit; integration test: an orchestrator status change at boot is recorded once via the layer (crosses the boot-thread boundary).

## 5. ADR + final fix-all gate

- [x] 5.1 Confirm `docs/adr/0041-tower-bus-middleware-spine.md` matches the shipped design (wrapper not full adoption; Ctx single-Arc; ADR-0037 bus-exclusivity clause superseded, byte-stream standard retained); added middleware-module + push-mechanism + streaming-non-goal sections.
- [x] 5.2 Backend gate green: `cargo test -p tillerd-orchestrator -p tillerd-desktop` (465 + 57 pass), `cargo clippy --all-targets -- -D warnings` clean, `sg scan` clear of new findings (message-dto fixed; remaining TS finding is pre-existing Phase-1 debt, not this change). Spec scenarios map 1:1 to tests. Full `bun run verify` + e2e deferred: blocked by pre-existing broken UI on this branch (Phase 1 not done) — backend-only change is fully green.
