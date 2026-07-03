# 0041. Tower as the bus middleware spine; lifecycle signals ride the bus for observation

- Status: accepted
- Date: 2026-06-25
- Supersedes: ADR-0037 (bus-exclusivity clause only; the zero-copy byte-stream event-dispatch standard is retained)

## Context

The orchestrator `Bus<Cx>` dispatches commands and queries with static generics and bakes its two cross-cutting concerns inline: a per-call `tracing` span and `.inspect_err(record)` for one structured error event. There is no composable place to add or reorder concerns. Notification recording sits outside the bus in a rejected desktop-side mpsc recorder, and the two lifecycle signals that should drive notifications (`surface_started`, `orchestrator_status`) are off-bus, so no bus-level observer can see them.

ADR-0037 (accepted three days prior) introduced a synchronous zero-copy borrowed-event dispatch standard for daemon-to-host **byte streams** and stated, as one clause, that "the CQS Bus stays execute/query only; streams are dispatch, not bus messages." A review gate for this change found that the existing sink-wrapping mechanism already provides composable layers and recommended extending it (RETHINK). The decision-maker reviewed that evidence and chose to adopt `tower` as the bus middleware spine instead, accepting the trade-offs below. This ADR records that decision and the single clause of ADR-0037 it revises.

## Decision

Adopt `tower` (`Service` + `Layer`) as the composition spine for the command/query bus.

- **The bus keeps its typed surface.** `execute<C: Command<Cx>>` and `query<Q: Query<Cx>>` remain the public API; there is no `dispatch_cqs(action)` entrypoint and no action enum erasing the command set. Internally, each call builds a small erased operation envelope (action name, command-vs-query kind, the handler call as a boxed future) and runs an ordered `Layer` stack around the handler. Type-erasure is bounded to one boxed future per dispatch, not the whole command set.

- **The layer stack composed at `build_bus`** is, outermost to innermost: error-logging -> notification-recording -> handler. Error-logging replaces the inline span + `inspect_err(record)`, preserving the one-structured-error-event-with-stable-code behavior. Inserting, removing, or reordering a layer is a `build_bus` edit only; no command, handler, or call site changes.

- **Layers live in a `middleware/` module.** Each layer is a file under `crates/orchestrator/src/middleware/` (`error_logging`, `notification_recording`); the layer order is declared in exactly one place, `middleware::pipeline()`, which dispatch calls and `build_bus` wires the dependencies for. The bus dispatcher (`Bus`, `Op`, `HandlerService`) stays in `shared/bus.rs`.

- **`Notable` is read at the typed boundary, not inside a layer.** `execute_notable<C: Command<Cx> + Notable>` computes `c.notification()` into `Op.notable` before the stack runs; ordinary commands keep using `execute`/`query` with no `Notable` bound. The recording layer consumes the already-extracted `Option<RecordNotification>`.

- **Live UI push retains ADR-0037 event-dispatch.** The recording layer is the sole writer: it persists once, then announces the persisted record on a `Broadcast<dyn NotificationSink>` (event-dispatch, off bus). The host subscribes a forwarder that converts to its wire shape and emits to the renderer — no second record, no command/wire rebuild, no renderer change.

- **Lifecycle signals ride the bus as observable operations.** `surface_started` and `orchestrator_status` become bus operations carrying primitive data and implementing a `Notable` marker trait exposed on the envelope. The notification-recording layer is the single point that turns an observed `Notable` signal into exactly one recorded notification. Producers dispatch the signal through the bus rather than calling the desktop recorder. Boot-phase progress (`orchestrator://status`) stays a direct host emit — it is UI progress, not a notification.

- **Transaction and validation stay in command handlers.** No transaction-isolation or validation layer; the per-handler `cx.transaction()` boundary and DTO validation are unchanged. Auth and metrics are reserved, unbuilt layer slots.

- **Raw runtime I/O stays off the bus.** Surface input/resize/attach never pass through the layered dispatch path; no keystroke or raw input payload is captured by a layer.

- **Non-goal — client-provided streaming subscriptions.** A `subscribe`/`stream` verb where the client provides an async sink (desktop `ipc::Channel`, server WebSocket) and the orchestrator streams over a host-agnostic sink port is a separate change, not this one. It composes with the tiers above (internal `Broadcast` bridged to a client sink at the IPC edge) but introduces a new verb, a new port, and its own ADR.

- **ADR-0037 revision.** Only its bus-exclusivity clause is revised: discrete, low-rate lifecycle *signals* may ride the bus for observation. ADR-0037's borrowed-event, synchronous, zero-copy standard for daemon-to-host **byte streams** (surface output via `Broadcast<dyn SurfaceSink>`) stands unchanged.

## Consequences

- One composable middleware mechanism on the bus; new cross-cutting concerns become layers without touching handlers. The same `tower` spine is available to the expected pre-v1 server host.
- The bus path gains one boxed handler future per dispatch where it previously never boxed. The cost is bounded to the handler call and negligible against the in-process SQLite work it wraps; accepted.
- A new `tower` workspace dependency (orchestrator crate), against the project's prefer-no-new-deps convention; justified by the server-host composition spine.
- Notification recording has a single home (the layer); the desktop mpsc recorder is retired and double-recording is structurally prevented.
- `tower::Service::call` returns a `'static` future while handlers borrow `&Cx`; the wrapper resolves this by owning a `Ctx` clone in the future. `Ctx` is collapsed to a single `Arc<CtxInner>` so that clone is one atomic increment per dispatch. The typed surface signature is preserved. Handlers stay plain `Command<Cx>`/`Query<Cx>` and do not become tower Services — the adapter sits only at the bus boundary, mirroring how axum/tonic adapt plain handlers while keeping real tower layers.
- Two ADRs now describe the dispatch landscape: ADR-0037 owns daemon-to-host byte streams (off bus), ADR-0041 owns command/query plus lifecycle-signal observation (on bus). The split is explicit to avoid reading all of ADR-0037 as superseded.
