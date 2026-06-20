## Context

ADR-0035's R1a relayered the orchestrator into `entities/` + `infra/` + `store/`, but parked one
cross-aggregate coordinator (`create_session`) at the store root and left the session-open
sequence (create-then-activate) assembled inside the tauri controller `workspace_host::session_create`.
Design D7 of `restructure-store-layers` named this temporary and committed R2/R3/R4 to formalize an
`app/` use-case layer. This change is that step. It is behavior-preserving: the existing unit,
integration, and e2e suites are the guard. The tauri command contract (`session_create` and peers)
is a frozen seam (0.0.6) and stays byte-identical.

## Goals / Non-Goals

**Goals:** a host-agnostic `app/` use-case layer owning cross-aggregate session/surface/launch
coordination; `create_session` promoted out of `store/`; the session-open sequence pulled out of the
tauri controller; controllers reduced to IPC shims; coordination unit-testable without a daemon.

**Non-Goals:** postgres backend; relocating `SurfaceApi`'s internal spec methods (`spawn_surface`,
`remove_surface`, `update_spec`) — it stays the surface-runtime port; moving `launch/spec.rs` or
`launch/executor.rs` (domain utilities); any behavior change, new dependency, or new crate.

## Decisions

### D1 — `app/` module in `crates/orchestrator`

New `crates/orchestrator/src/app/` with `mod.rs` + `session.rs`. No new crate (layout-preference:
modules in existing crates). `lib.rs` gains `pub mod app;`. The layer depends downward on `store`
(per-entity stores) and on a narrow activation port — never on a host or the concrete runtime.

### D2 — `create_session` moves verbatim

The function moves from `store/storage.rs` into `app/session.rs` unchanged (body byte-for-byte:
resolve `template_id` -> `instantiate_for_session` -> `Sessions::create`). Its re-export drops from
`store/mod.rs`; it is re-exported as `app::create_session`. Pre-v1, no back-compat alias. The two
internal callers (the `surface/api.rs` test helper, `tests/store_architecture.rs`) update their
import to `app::create_session`. `Storage` and the rest of `store/storage.rs` stay put.

### D3 — `open_session` use case over a `SessionActivator` port

`app::open_session(draft, &LaunchTemplates, &Sessions, &impl SessionActivator) -> Result<Session>`
sequences: `create_session(...)`, then best-effort `activator.activate(&session.id)` whose error is
logged (`tracing::warn`) and swallowed, then returns the session. The session-open behavior is thus
identical to today's controller (create, then non-fatal launch).

`SessionActivator` is a one-method port defined in `app/`, using RPITIT (`fn activate(&self, id:
&SessionId) -> impl Future<Output = Result<()>>`) — mirroring the existing `SurfaceLauncher` trait,
no `async-trait`, no `dyn` (ADR-0035). `SurfaceApi` implements it by delegating to its existing
`launch_session` and discarding the per-item result vec (the controller already ignores it). The
port keeps `app/` independent of the surface runtime: production wires `SurfaceApi`; unit tests wire
a trivial fake — no fake daemon needed.

*Why a port, not a concrete `&SurfaceApi`:* `SurfaceApi` owns a live runtime socket, so depending on
it concretely would force a daemon-backed test for the use case and couple `app/` to surface infra.
The port has two real impls (`SurfaceApi` + test fake) and expresses the application-layer/infra
boundary — not an ornamental abstraction.

### D4 — `workspace_host::session_create` becomes a shim

The command keeps its frozen signature and `SessionResponse` return. Its body maps the flat args
into a `NewSession` draft and calls `app::open_session(draft, &storage.launch_templates,
&storage.sessions, &surfaces.api)`. The inline `create_session` + best-effort `launch_session` +
`eprintln` block is removed (the best-effort log now lives in the use case via `tracing::warn`).
`do_session_create` collapses into the command (or stays as a thin draft-builder if a test needs it).

### D5 — Tests

`create_session`'s contract tests move with it (import update only, assertions unchanged).
`open_session` gets unit tests over the fake activator: (a) persists + invokes the activator once;
(b) activation error is non-fatal, session still returned. Each spec scenario maps 1:1 to a unit
test. The existing surface integration test and desktop e2e cover the host-delegation path; they
stay green unchanged, proving behavior preserved.

## Risks / Trade-offs

- **eprintln -> tracing::warn for the best-effort launch log** → channel changes (stderr line ->
  structured tracing); user-observable behavior is unchanged and a library logging via `tracing` is
  the correct layering. Mitigation: documented here; the existing suites assert session outcome, not
  log channel.
- **Behavior drift hidden in the move** → guarded by the unchanged unit + integration + e2e suites;
  any test needing a real edit beyond an import signals the move changed behavior — stop and fix.
- **New port trait** → minimal (one method, RPITIT, two real impls); justified by host-agnosticism
  and daemon-free testing, consistent with the existing `SurfaceLauncher` pattern.

## Migration Plan

Incremental, `cargo test` green between steps: add `app/` with `create_session` moved + re-exports
rewired + caller imports updated (suite green) -> add `SessionActivator` + `open_session` + its unit
tests -> impl `SessionActivator` for `SurfaceApi` -> rewire the tauri command to delegate -> full
verify. Pre-v1, no data migration. Rollback: revert branch.

## Open Questions

None blocking. Deferred by design: postgres backend; any deeper relocation of `SurfaceApi`'s spec
methods into `app/`.
