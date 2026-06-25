## 1. Executive Summary

- **Status**: RETHINK (verdict) → **OVERRIDDEN by user; proceeding with tower per Action Item (C)** (2026-06-25)
- **Reviewer**: AI Architect (OpenSpec Reviewer)
- **Target Proposal**: tower-bus-middleware / proposal.md
- **Summary**: The goal — composable cross-cutting layers on the bus plus on-bus observation of lifecycle signals — is sound, but adopting `tower` as the mechanism fights three independent facts in the current codebase: the bus's documented no-box static dispatch, a 3-day-old *accepted* middleware mechanism (ADR-0037 sink-wrapping over `Broadcast`) that already delivers composable layers and is the designated home for lifecycle streams, and a new dependency the project deliberately avoids. The same outcome is reachable by extending the existing mechanism, with no tower, no new dep, and no supersession of ADR-0037. **The user reviewed this finding and chose to keep tower anyway (Action Item C); design/specs proceed on the tower path with the obligations C lists carried into the proposal.**

## 2. Research & Evidence

### Existing Logic / Utilities

- `Broadcast<S: ?Sized>` (`shared/bus.rs:29-57`): thread-safe synchronous 1:N fan-out; `subscribe(Arc<S>)` + `dispatch(|s| …)`. The generic, reusable composition core.
- `impl SurfaceSink for Broadcast<dyn SurfaceSink>` (`events/surface.rs:38`): the fan-out **is itself a sink** — sinks compose by nesting.
- Blanket `impl<F> SurfaceSink for F` (`events/surface.rs:46`): any closure is a sink subscriber (closure-over-primitives, the host's edge).
- `impl SurfaceSink for Recorder` (`events/surface.rs:64`): **a recording middleware that wraps a sink already exists** — the exact "record-on-event" shape the proposal wants to build with tower.
- `Bus<Cx>` (`shared/bus.rs:62-95`): `execute<C: Command<Cx>>` / `query<Q: Query<Cx>>`, static dispatch, span + `inspect_err(record)`. File header (`:1-5`): *"it never boxes (dispatch is static over the concrete operation type)."*

### Related Patterns

ADR-0037 (accepted 2026-06-22) — *"Synchronous zero-copy event dispatch: borrowed events, fan-out terminal, middleware by wrapping"* — is a hand-rolled, tower-Layer-equivalent purpose-built for this codebase three days ago:
- *"Middleware composes by wrapping. A layer (telemetry, filter, rate-limit) implements the sink trait and wraps an inner `Arc<dyn DomainSink>`. Inserting or removing a layer never changes a producer emit call site."* (ADR-0037 Decision) — this is precisely a `Layer` stack.
- *"Every future outbound stream (service health, **daemon lifecycle, supervision**, task progress) would clone the same ad-hoc shape"* → standardized via `Broadcast` (ADR-0037 Context). `orchestrator_status` and `surface_started` are daemon-lifecycle/supervision signals — **named, designated instances of this mechanism**, not new territory.

### Potential Conflicts

1. **Structural impedance.** `tower::Service<Request>` requires a single concrete `Request` type with one `Response`/`Error`. The bus dispatches a *family* of heterogeneous command/query types via static generics. Making `Bus` a `tower::Service` forces either (a) type-erasure to one `Request` enum with boxed handlers — which the bus explicitly forbids ("never boxes") — or (b) one `Service` per command type, which gives no shared layer stack and defeats the purpose. ADR-0037 already rejected the isomorphic idea: *"A generic `Emitter<E>` storing the event type is rejected: a borrowed enum is a family of types and cannot be a stored `'static` parameter."*
2. **ADR conflict.** ADR-0037 Decision: *"The CQS `Bus` stays `execute`/`query` only; streams are dispatch, not bus messages."* Routing lifecycle *signals* onto the command bus contradicts an accepted ADR and would require superseding it — three days after it landed.
3. **Network-shaped benefits, in-process bus.** tower's reusable layers (timeout, retry, load-shed, rate-limit, concurrency-limit) solve network/latency problems. The bus is in-process local IPC (~0 latency per the client-engine notes). The layers that pay for tower don't apply here; the two layers actually wanted (error-log, notification-record) are trivially hand-rolled and one already exists (`Recorder`).
4. **New dependency.** `tower` is absent from the workspace; project convention (crate-layout-preference) avoids new deps without a hard reason.

### Code Evidence

```text
events/surface.rs:38  impl SurfaceSink for Broadcast<dyn SurfaceSink> { … }   # fan-out is a sink
events/surface.rs:46  impl<F> SurfaceSink for F where F: Fn(&str, &SurfaceEvent) … # closure sinks
events/surface.rs:64  impl SurfaceSink for Recorder { … }                     # recording middleware already exists
shared/bus.rs:1-5     //! it never boxes (dispatch is static over the concrete operation type)
ADR-0037:81           The CQS Bus stays execute/query only; streams are dispatch, not bus messages.
# rg "tower" over all Cargo.toml -> no matches (dependency absent)
```

### Search Keywords Attempted

`Broadcast<`, `impl .* Sink for`, `wraps`, `inner: Arc<dyn`, `tower` (Cargo.toml), `middleware|interceptor|Layer` (orchestrator src), `Io` trait, `record(&Error)`, `surface_started`, `orchestrator_status`, `spawn_recorder`.

## 3. Alternative Analysis

| Approach | Pros | Cons | Complexity |
| :-- | :-- | :-- | :-- |
| **Proposed** (tower `Service` + `Layer` on `Bus`) | Mature ecosystem; standard for a future network-server host; reusable timeout/retry layers | Forces boxing/type-erasure the bus forbids; heterogeneous-static-dispatch mismatch; supersedes accepted ADR-0037; duplicates the existing wrapping-middleware; new dep; network-shaped benefits don't pay in-process | High |
| **Alt A** (extend ADR-0037 sink-wrapping to the bus; lifecycle signals → `Broadcast`; notification-record + error-log as wrapping layers) | Reuses an accepted 3-day-old mechanism; zero new deps; no ADR conflict; lifecycle signals reach their *designated* home; `Recorder` middleware already exists; preserves no-box/zero-copy | Hand-rolled (no off-the-shelf layer crates); a small bus-side telemetry-layer trait is new | Low–Med |
| **Alt B** (minimal: route lifecycle signals onto `Broadcast` + add a recording sink; leave bus telemetry inline) | Smallest diff; directly kills the rejected desktop mpsc recorder (b980eac); no bus rewrite | Does not deliver the "composable bus layer stack" the proposal asks for; error-logging stays inline in `execute`/`query` | Low |

### Conclusion

**Alt A** is the best fit: it delivers the proposal's actual goal — composable cross-cutting layers plus on-bus-observable lifecycle signals and a single notification-recording point — by extending the middleware mechanism the project just accepted (ADR-0037), with no tower dependency, no structural fight with static dispatch, and no supersession of a fresh ADR. If the "composable bus telemetry stack" is judged not yet needed, **Alt B** ships the rejected-recorder fix alone. The proposed tower path buys ecosystem layers whose value is network-shaped and unrealized by an in-process bus, at the cost of every conflict above.

## 4. Feasibility Check

- [x] **Dependency Check**: `tower` is NOT in the workspace (proposed path adds it). Alt A / Alt B need **zero** new deps — `Broadcast`, `SurfaceSink`, and a `Recorder` sink already exist.
- [x] **Performance Impact**: Proposed path's type-erasure adds per-call boxing/allocation on a path the bus deliberately keeps box-free; tower readiness/poll machinery is overhead the in-process bus doesn't need. Alt A/B preserve synchronous zero-copy fan-out (ADR-0037), no unbounded buffering.
- [x] **Testability**: Alt A/B test exactly like the existing `Broadcast`/`Recorder`/bus-error tests already in `shared/bus.rs` and `events/surface.rs` (subscriber-order, record-on-event, one-error-event). The proposed tower path needs new Service/poll-readiness test scaffolding with no existing precedent in-repo.

## 5. Detailed Verdict & Action Items

### Verdict

**RETHINK.** The proposal's *objective* is correct and worth doing; its chosen *mechanism* (adopt tower; make `Bus` a tower `Service`; route signals onto the command bus) is the wrong instrument for this codebase. Three load-bearing assumptions are off:

1. *"There is no composable place for cross-cutting concerns."* — There is: ADR-0037's sink-wrapping over `Broadcast`, accepted three days ago, with a working `Recorder` middleware.
2. *"tower fits the bus."* — The bus is heterogeneous static dispatch that never boxes; `tower::Service` needs a uniform typed request and pushes toward boxing. Documented-invariant conflict.
3. *"Lifecycle signals should ride the command bus."* — ADR-0037 explicitly designates `Broadcast` fan-out (not the bus) as the home for daemon-lifecycle/supervision streams, the exact category of `surface_started`/`orchestrator_status`.

### Action Items

**Decision required from the user (this contradicts an explicit prior choice).** In planning, the user explicitly selected *"full tower middleware rewrite"* and *"adopt tower."* The evidence above is the review gate's honest finding that the codebase already provides the mechanism and that tower fights three documented constraints. This is surfaced, not auto-overridden — the user owns the call:

- **(A) Accept RETHINK → pivot to Alt A.** Rework proposal.md: drop the `tower` dependency and the `Bus`-as-`tower::Service` rewrite; reframe as *extend the ADR-0037 sink-wrapping middleware to the command bus*; route `surface_started`/`orchestrator_status` onto `Broadcast` as lifecycle events; implement notification-recording and error-logging as wrapping layers; **no ADR-0037 supersession** (additive). Then proceed to specs/design.
- **(B) Accept RETHINK → pivot to Alt B** (minimal) if the composable bus-telemetry stack is deferred: route lifecycle signals onto `Broadcast` + add the recording sink, kill the b980eac desktop recorder, leave bus telemetry inline.
- **(C) Override the verdict and keep tower** with eyes open: then proposal.md must add an explicit ADR superseding ADR-0037, justify the new dep against crate-layout-preference, and own the boxing/type-erasure cost. Re-run review only to confirm the override is recorded, not to re-litigate.

Recommended: **(A)**.

### Override Record (2026-06-25)

User chose **(C) — keep tower**, eyes open, after reviewing the RETHINK evidence. Binding obligations carried into proposal.md and design:

1. A new ADR (next number 0041) **supersedes ADR-0037's** "Bus stays execute/query only; streams are dispatch, not bus messages" clause, for the lifecycle-signal-observation case. ADR-0037's zero-copy event-dispatch standard for *daemon-to-host byte streams* (surface output) is NOT discarded — only the bus-exclusivity clause is revisited.
2. The `tower` workspace dependency is justified against crate-layout-preference in proposal Impact (hard reason: tower `Service`/`Layer` is the standard composition spine for the expected pre-v1 network-server host; one mechanism for desktop + server).
3. The boxing / type-erasure cost on the previously no-box static-dispatch bus is acknowledged as accepted, to be designed in design.md (the `Service` request representation — erased enum vs per-type Service — is a design decision, not re-opened here).

Design/specs proceed. This override is recorded, not re-litigated.

## 6. Review Metadata

- **Review Date**: 2026-06-25
- **Context Depth**: 6+ files read (`shared/bus.rs`, `shared/message.rs`, `boot.rs`, `events/surface.rs`, `notification_host.rs`, ADR-0037), 3 prior investigator sweeps, ~8 searches
- **Tools Used**: Read, Grep (rg), Bash, Agent (Explore investigators)
