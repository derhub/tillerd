## Context

The orchestrator `Bus<Cx>` (`crates/orchestrator/src/shared/bus.rs`) dispatches commands and queries with static generics (`execute<C: Command<Cx>>`, `query<Q: Query<Cx>>`) and bakes its two cross-cutting concerns inline: a `tracing` span per call and `.inspect_err(record)` for one structured error event. There is no composable place to add or reorder concerns. Notification recording lives outside the bus entirely, in a rejected desktop-side mpsc recorder (`apps/desktop/src-tauri/src/notification_host.rs`, b980eac), and the two lifecycle signals that should drive notifications (`surface_started`, `orchestrator_status`) are off-bus — one on the surface-create command path emitting to the host directly, one a boot-thread `app.emit`.

This change adopts `tower` as the middleware spine (user decision; the review gate returned RETHINK and was overridden — see `review.md` Override Record). Two clarifications from planning narrow the scope below the original diagram:

- The bus keeps its **typed** `execute<C>` / `query<Q>` surface. No single `dispatch_cqs(action)` entrypoint, no action enum, no collapse of the 111 per-command Tauri shims, no client-bindings rework — those stay as-is and keep driving the bus.
- **Transaction and validation stay inside command handlers** (the 6 `cx.transaction()` sites and per-handler DTO validation are unchanged). No transaction-isolation layer, no validation layer.

So the layer stack built here is **error-logging + notification-recording**, composed at `boot.rs::build_bus`, with reserved (unbuilt) slots for future auth/metrics/tracing layers. Constraints carried in: the bus must not capture surface input/resize/attach payloads (no keystroke logging — `bus.rs` invariant); ADR-0037's zero-copy byte-stream event-dispatch standard is retained; entities/infra/events stay crate-internal.

## Goals / Non-Goals

**Goals:**

- Re-express `Bus` dispatch as a `tower::Service` wrapped by an ordered `tower::Layer` stack composed once at `build_bus`.
- Move the error-logging concern (one structured error event with the stable code) out of inline `inspect_err` into a layer, preserving the exact OTel field shape and the existing bus tests.
- Add a notification-recording layer as the single point where an observed lifecycle signal becomes a recorded notification, retiring the desktop mpsc recorder (b980eac).
- Route `surface_started` and `orchestrator_status` through the bus as observable operations so the recording layer sees them.
- Keep the typed `execute<C>`/`query<Q>` surface API source-compatible for the 111 existing shims.

**Non-Goals:**

- The single `dispatch_cqs(action)` IPC entrypoint and any action-enum type-erasure of the whole command set.
- Collapsing the per-command transport shims or reworking the client-bindings TypedKey system (follow-on thin-tauri change).
- Dissolving `apps/desktop/src-tauri/src/*_host.rs` and the Phase 1 UI migration.
- Transaction-isolation, validation, auth, or metrics layers (auth/metrics are reserved slots only; tx/validation stay in handlers).

## Decisions

### D0. Collapse `Ctx` to a single `Arc<CtxInner>` (prep)

Before the Service wiring, change `Ctx` from a 4-field `#[derive(Clone)]` struct to a newtype over one `Arc<CtxInner>`:

```rust
#[derive(Clone)]
struct Ctx(Arc<CtxInner>);
struct CtxInner { db: SqlitePool, kv: SqliteKv, fs_root: PathBuf, runtime: Runtime }
```

Accessors (`db()`, `kv()`, `fs_root()`, `runtime()`, `runtime_arc()`) forward through the inner. The wrapper (D1) must own a `Ctx` clone inside each one-shot future (the `'static` future cannot borrow `&Cx`); with this collapse that clone is **exactly one atomic increment, zero allocation** per dispatch, instead of today's ~3 atomics + a `PathBuf` heap allocation. This is the standard tower idiom for shared state (axum/tonic: state behind one `Arc`, cheap-clone per request).

### D1. Wrapper adapter, not handler-level Service adoption; erase only at the layer boundary

`execute<C: Command<Cx>>` and `query<Q: Query<Cx>>` stay as the public bus API, and the ~115 handlers stay plain `Command<Cx>`/`Query<Cx>` — **they do not become `tower::Service`s.** A generic bus-level adapter turns each dispatch into a one-shot tower `Service` driven through a real tower `Layer` stack (`ServiceBuilder` + `ServiceExt::oneshot`). Internally each call builds a small **erased operation envelope** as the Service `Request`: the action name (`std::any::type_name::<C>()`), the operation kind (command vs query), an optional `&dyn Notable`, and the handler invocation as a boxed future producing the typed result. Layers operate on the envelope (name, kind, notable, outcome), never on the concrete command type.

- **Why a wrapper, not full adoption:** `tower::Service<Request>` is monomorphic in `Request` — one Service, one request type. Heterogeneous typed dispatch over ~115 commands does not fit one Service; full adoption would force either the rejected single-enum erasure (`dispatch_cqs`) or 115 per-type Services whose `poll_ready`/backpressure is meaningless in-process. The wrapper is the idiomatic tower integration: axum handlers and tonic RPCs are not Services either — the boundary adapts plain handlers into a Service while the *layers* are real tower. We keep the full Layer ecosystem (timeout, concurrency-limit, etc.) without touching a single handler.
- **Why erase at the boundary:** bounds type-erasure to one boxed future per dispatch and gives one envelope type, one set of concrete layer Services, and no 115× monomorphization of the stack. The typed surface and all 111 shims compile unchanged.
- **Alternative considered (rejected):** a single `Request` enum over all ~115 operations (the `dispatch_cqs` shape) — rejected by the user ("keep the bus like design"); forces the shim/bindings rework that is out of scope.
- **Alternative considered (rejected):** make each handler a `tower::Service` — 115-handler rewrite, per-type stack monomorphization, meaningless `poll_ready`, no added benefit for in-process dispatch.

### D2. The layer stack and its order

Composed at `build_bus` outermost→innermost: **error-logging → notification-recording → handler**. Error-logging is outermost so it observes failures from any inner layer too. Notification-recording observes the operation and its outcome and records when the operation is a lifecycle signal. The tracing span moves into the error-logging layer (or a thin span layer it owns) so the per-call span is still present.

- **Why this order:** logging must see everything including a recording-layer fault; recording must see the handler's success/failure to record accurately.
- The stack is built once and held in `Bus`; inserting/removing a layer is a `build_bus` edit only, no handler or call-site change (spec requirement).

### D3. Lifecycle signals as `Notable` bus operations

`surface_started` and `orchestrator_status` become bus operations carrying primitive data (session id, surface id, ready/error). They implement a small marker trait — `trait Notable { fn notification(&self) -> Option<NotificationWire>; }` — exposed on the erased envelope as `&dyn Notable`. The notification-recording layer calls `notification()` and, when `Some`, records exactly one notification. Producers (the surface-create path; the boot sequence) **dispatch the signal through the bus** instead of calling the desktop recorder or `app.emit` for notifications.

- **Why:** keeps recording in one layer (single-recording-point spec requirement) while the typed signal carries its own data; the layer reads it through a trait, not a stringly-typed action match. Only lifecycle signals impl `Notable`, so the surface area is tiny.
- **Boot-phase status** (`emit_status` Booting/Ready/Failed on `orchestrator://status`) stays a direct `app.emit` — it is UI progress, not a notification; only the notification-worthy status change becomes a `Notable` bus operation.
- **Alternative considered (rejected):** route signals through ADR-0037's `Broadcast` fan-out and record there (the review's Alt A). Rejected by the tower override; recorded for history in `review.md`.

### D4. ADR-0041 supersedes ADR-0037's bus-exclusivity clause only

ADR-0037 states "the CQS Bus stays execute/query only; streams are dispatch, not bus messages." ADR-0041 revises that one clause so lifecycle *signals* (discrete, low-rate, not byte streams) may ride the bus for observation. ADR-0037's zero-copy borrowed-event standard for daemon-to-host **byte streams** (surface output via `Broadcast<dyn SurfaceSink>`) is retained unchanged.

### D5. tower dependency scope

Add `tower` to `crates/orchestrator/Cargo.toml` only (with the `Layer`/`Service`/`util` features actually used). Justified against crate-layout-preference: tower is the standard composition spine for the expected pre-v1 server host; adopting it in the bus gives one mechanism across desktop and server.

## Risks / Trade-offs

- **Boxing on a previously box-free path** -> one boxed handler future per dispatch. Mitigation: bounded to the handler call (not per-layer); the bus path is in-process local IPC dominated by SQLite work, so the alloc is negligible; documented as accepted in the ADR.
- **`async` in a `tower::Service` over borrowed `Cx`** -> lifetime friction between `Service::call` returning a `'static` future and the handler borrowing `&Cx`. Mitigation: the wrapper owns a `Ctx` clone inside the future; after the D0 collapse that clone is one atomic increment. The typed surface signature is preserved.
- **Partial ADR supersession confusion** -> a reader thinks all of ADR-0037 is dead. Mitigation: ADR-0041 states explicitly that only the bus-exclusivity clause is revised and the byte-stream standard stands.
- **Two notification paths during migration** -> the desktop recorder and the new layer both record. Mitigation: retire the desktop recorder (b980eac) in the same change; the single-recording-point spec scenario guards against duplicates.
- **`Notable` coupling** -> commands gain a trait impl. Mitigation: only the two lifecycle signals impl it; ordinary commands are untouched.

## Migration Plan

1. Collapse `Ctx` to `Arc<CtxInner>` (D0), forwarding accessors; existing `Ctx` tests stay green.
2. Add `tower` dep (orchestrator crate).
3. Introduce the erased operation envelope + the wrapper adapter (D1) + the `Layer`/`Service` wiring inside `execute`/`query`, keeping signatures; port the existing span + error-event into an error-logging layer. Bus tests (`a_command_error_logs_exactly_one_error_event...`, `a_successful_operation_logs_no_error_event`) stay green unchanged.
3. Define `Notable` + the notification-recording layer; make `surface_started`/`orchestrator_status` `Notable` bus operations; dispatch them from their producers.
4. Compose the stack in `build_bus`.
5. Retire the desktop mpsc recorder (b980eac) and the off-bus notification emit; keep `orchestrator://status` boot-progress emits.
6. Write ADR-0041 to `docs/adr/`.

Rollback: the change is additive behind the typed surface; reverting the `build_bus` composition + restoring inline `inspect_err` returns the prior behavior. Pre-v1, no external contract is frozen by this (internal seam).

### D6. Layers live in a `middleware/` module with a single-source registry

Layers are files under `crates/orchestrator/src/middleware/` (`error_logging`, `notification_recording`); `middleware::pipeline()` is the one place the order is declared, called by `drive()` and wired by `build_bus`. The dispatcher (`Bus`, `Op`, `HandlerService`, `Notable`, `Broadcast`) stays in `shared/bus.rs`. New cross-cutting concerns add a file + a line in `pipeline()`.

## Open Questions

- The exact `tower` feature set and the `Service` future representation — resolved in APPLY: `Future = BoxFuture<T>` with an explicit `Send` bound (the desktop transport needs `Send` futures); does not change the spec contract.
- Whether the error-logging span belongs in its own thin span layer or inside the error-logging layer — resolved: kept inside the error-logging layer, preserving the existing span field shape.
- **Follow-on change `bus-subscribe-streams` (out of scope here):** a `subscribe`/`stream` verb where the client provides an async sink (desktop `ipc::Channel`, server WebSocket) over a host-agnostic sink port. Composes with this change's tiers (internal `Broadcast` bridged to the client sink at the IPC edge per ADR-0037's anticipated IPC-boundary copy). New verb + port + ADR — its own unit.
