# 0034. State model as a shared contract

- Status: proposed
- Date: 2026-06-19

## Context

Entity lifecycle, surface status, and action-guarding are currently expressed ad hoc: a
free-form `last_status` string on the surface, implicit lifecycle transitions, and guard logic
duplicated across the orchestrator and the client where they can drift. Multi-window and
optimistic UI need one agreed model of what state an entity is in, which transitions are legal,
and who has authority — shared by both sides without codegen.

Ships with ADR-0033 in 0.0.15 so `state.db` lands its final typed schema once (the typed
runtime columns are this model's persistence).

## Decision

Make the state model a single declarative contract, loaded by both sides.

- **`contracts/state-model.json` (+ `.schema.json`)** — single source. Rust loads via
  `include_str!` + serde; TS imports + zod. No codegen. The contract marks which states are
  persistable vs runtime-only.
- **Lifecycle FSM.** Shared CRUD: `Creating -> Active -> Archiving -> Archived -> Deleting`.
  Surface special: `Spawning -> Attaching -> Live -> Closing -> Closed`.
- **Surface status split.** Runtime `ProxyState` (`Spawning`/`Attaching`/`Closing`, in-memory,
  rebuilt at boot via `resume_all`) vs persisted typed `last_status` (`Live | Exited |
  Crashed`, `state.db`) that gates resume-on-boot. Replaces the free-form string.
- **Sync status.** `Confirmed | Pending | Rejected | Stale | Conflicted`; optimistic with
  rollback; pending is in-memory only. `Conflicted` locks the entity until resolved —
  per-node for `layout.json`, per-entity for flat files.
- **Guards.** `*-ing` states are locked; only stable states accept actions. The orchestrator
  enforces; the client is advisory. A contract test (modeled on `command_contract.rs`) proves
  UI and server guards agree.
- **View pointers.** Minimal global seed in `state.db`: `activeWorkspace` (new-window seed),
  `sidebar.expanded.<proj>`, `lastSession.<proj>`; resolved against live lifecycle. Per-window
  context comes from URL intent (in-memory; restore-after-quit deferred); `focusedLeaf`
  in-memory. Workspace activity is a derived runtime read-model (rollup of surface
  `ProxyState`), keyed by workspace id, never a domain field.

## Consequences

- One source of truth for legal states/transitions; UI and server cannot silently diverge
  (contract test fails on drift).
- The client (0.0.16) wires sync status through TanStack Query (the sync axis) and guards
  through Store; the orchestrator emits `changed{id}` so windows invalidate the matching Query
  key.
- Adding a state or transition is a contract edit + a test, not parallel edits in two
  languages.
- Depends on ADR-0033 for the operational plane that persists the typed columns.
