# 0044. State model re-based: Rust-authoritative contract, stable states only

- Status: accepted, supersedes ADR-0034
- Supersedes: ADR-0034
- Date: 2026-07-03

## Context

ADR-0034 (proposed, never implemented) defined the state model as a shared
`contracts/state-model.json` loaded by both sides, a lifecycle FSM with transient `*-ing`
states, a five-value sync-status enum with conflict locking, and view pointers in
`state.db`. Its substrate is gone: ADR-0036 deleted `state.db` and the file-merge sync
world, made entities own behavior (typed status enums and `guard_*` methods now live in
the entity layer), and made every mutation a single per-command sqlite transaction.
ADR-0039 made the client's server-state cache the sync axis. The deferred concerns —
guards on the client, view pointers, workspace activity — still need a home; the shape
ADR-0034 gave them no longer fits.

## Decision

Re-base the state model onto the de-abstracted architecture:

- **Rust entities are the single source of state and guard truth.** The entity layer
  exports its states, legal transitions, and guard rules as machine-readable tables. A
  committed JSON fixture generated from those tables is asserted by a Rust test and by a
  TS test against the client's typed mirror — drift on either side fails the build. There
  is no authored shared contract file and no codegen.
- **Stable states only.** No transient `*-ing` lifecycle states: per-command transactions
  make them unobservable locally. The contract covers the existing enums
  (`Active ↔ Archived`; surface `Pending → Live → Idle/Failed`).
- **Sync status is the cache's native axis.** Pending/error/stale from the server-state
  cache replaces ADR-0034's `Confirmed|Pending|Rejected|Stale|Conflicted`; conflict
  locking is dropped with the file-merge model that motivated it.
- **Guards: server enforces, client advises.** The client disables actions from the
  mirrored guard table, evaluated on fields its read models already carry; a bypassed
  guard still fails server-side through the typed error path.
- **View pointers live in the settings store** (global scope, per-target keys:
  `view.active-workspace`, `view.last-session.<project>`,
  `sidebar.expanded.<project>`), read through the server-state cache, written
  optimistically, resolved against live lifecycle at their consumption point.
- **Workspace activity is a server-derived query**, one SQL aggregate over persisted
  surface status — never a domain field — kept live by a `surface_status_changed` event
  on the existing dispatch spine and subscription transport.

## Consequences

- One reviewable fixture diff per state-model change; UI and server cannot silently
  diverge (both tests fail on drift).
- Client enablement logic collapses into one mirror module instead of per-component
  conditionals; server authority is unchanged.
- View pointers survive webview-storage wipes and are visible to future hosts; the
  webview `localStorage` copy of active-workspace and sidebar expansion is removed.
- Activity badges read one query per window, invalidated by push — no client-side
  session/surface fan-out.
- Remote/web hosts with real latency may eventually want transient states or richer sync
  status; that would be a new ADR superseding this one, not a revival of ADR-0034.
