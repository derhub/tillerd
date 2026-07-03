## Why

The orchestrator `Bus<Cx>` hand-rolls its cross-cutting concerns: a `tracing` span plus `.inspect_err(record)` baked into both `execute` and `query` (`crates/orchestrator/src/shared/bus.rs`). Notification recording lives even further out — in a desktop-side mpsc recorder (`apps/desktop/src-tauri/src/notification_host.rs`, commit b980eac) that the user has rejected. There is no composable place to add error-logging, notification-recording, metrics, or tracing as independent, ordered concerns, and the two notification triggers that matter (`surface_started`, `orchestrator_status`) are off-bus desktop signals a bus-level observer cannot see at all.

Adopt `tower` as the middleware spine: `Bus` dispatch becomes a tower `Service`, cross-cutting concerns become an ordered `Layer` stack composed once at `boot.rs::build_bus`, and the lifecycle signals are routed onto the bus so the stack can observe them. This is the foundation the thin-Tauri refactor (follow-on change) builds on.

## What Changes

- Add `tower` as a workspace dependency (new dep — justified in review; the user normally avoids new crates per project convention).
- **BREAKING (pre-v1, new seam):** `Bus<Cx>` dispatch is re-expressed as a tower `Service`. The hand-rolled `tracing` span + `record(&Error)` in `execute`/`query` are removed and re-homed as layers.
- New tower `Layer` stack composed at `boot.rs::build_bus`:
  - error-logging layer (replaces `inspect_err(record)` + the per-call span),
  - notification-recording layer (replaces the rejected desktop mpsc recorder of b980eac),
  - room for metrics / tracing layers without touching handlers.
- Route the off-bus lifecycle signals (`surface_started`, `orchestrator_status`) onto the bus as observable messages so the notification-recording layer is their single recording point — superseding the desktop-side recorder and the boot-thread `app.emit`/`emit_only` notification path.
- New ADR (0041) **supersedes ADR-0037's bus-exclusivity clause** — *"the CQS Bus stays execute/query only; streams are dispatch, not bus messages"* — for the lifecycle-signal-observation case, so the bus can carry and a layer can observe `surface_started`/`orchestrator_status`. ADR-0037's zero-copy event-dispatch standard for daemon-to-host byte streams (surface output) is retained, not discarded.

**Decision (resolved at the review gate, 2026-06-25):** the review gate returned **RETHINK** — it found the codebase already has a sink-wrapping middleware mechanism (ADR-0037 `Broadcast` + an existing `Recorder`), that the bus's documented no-box static dispatch fights `tower::Service`'s uniform-typed-request shape, and that the wanted layers are network-shaped. After reviewing that evidence the user **overrode the verdict and chose the tower path (review Action Item C)**. The obligations of that choice are folded into this proposal: the ADR-0037 supersession above, the dependency justification below, and the accepted boxing/type-erasure cost (the `Service` request representation is a design.md decision). See `review.md` Override Record.

## Out of scope (follow-on changes)

- Full dissolution of `apps/desktop/src-tauri/src/*_host.rs` into thin transport shims.
- The Phase 1 UI migration off `commands`/`events` onto `query`/`command`/`subscribe`.

## Capabilities

### New Capabilities

- `bus-middleware`: the orchestrator command/query bus as a tower `Service` wrapped by an ordered `Layer` stack (error-logging, notification-recording, extensible) composed at bootstrap; defines what a layer observes, layer ordering, and how lifecycle signals enter the bus to be observable.

### Modified Capabilities

None. The specs pass concluded neither `notification-center` nor `event-dispatch` changes at the spec level:

- `notification-center` — its user-facing requirements (feed, bell, "lifecycle signals become notifications", global feed) are unchanged; only the recording *mechanism* moves to the bus layer, which is an implementation detail captured by `bus-middleware`'s single-recording-point requirement.
- `event-dispatch` — the borrowed-event / fan-out / wrapping-middleware standard is retained for byte streams. The bus-exclusivity clause being superseded lives in ADR-0037, not in the `event-dispatch` spec, so the spec needs no delta. ADR-0041 records the supersession.

## Impact

- **Code:** `crates/orchestrator/src/shared/bus.rs` (Service rewrite), `crates/orchestrator/src/boot.rs::build_bus` (layer-stack composition), `crates/orchestrator/src/shared/message.rs` (Command/Query traits as Service requests), the lifecycle-signal producers (`surface_host.rs` surface_started, `orchestrator_host.rs` orchestrator_status), and removal/supersession of `notification_host.rs` recorder (b980eac).
- **Dependencies:** new `tower` workspace dependency. Justification against crate-layout-preference (which discourages new deps): tower's `Service`/`Layer` is the standard composition spine for the web/server host expected before v1 (`apps/server`, dormant); adopting it now gives one middleware mechanism across desktop and server rather than a bespoke layer trait that the server host would later re-evaluate. Accepted hard reason.
- **Cost owned:** the bus today is static-dispatch and "never boxes"; expressing it as a `tower::Service` introduces type-erasure/boxing of the command/query request. This cost is accepted; the exact request representation (erased request enum vs per-type Service) is resolved in design.md.
- **ADRs:** new ADR-0041 for the tower bus middleware decision, superseding ADR-0037's bus-exclusivity clause (zero-copy event-dispatch for byte streams retained).
- **Tests:** bus/layer unit tests; lifecycle-signal-observation tests; existing notification-center tests re-pointed at the layer.
