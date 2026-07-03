# Proposal: state-model-query-store

## Why

The last 0.0.16 item — "wire view pointers + state-model guards + workspace-activity
read-model through Query/Store" — rides ADR-0034, which is still `proposed` and written
against a world ADR-0036 deleted (`state.db`, free-form `last_status`, file-merge sync).
The client today has no guard awareness (actions the orchestrator will reject render as
enabled), no per-workspace activity signal (0.0.20 status badges have nothing to read),
and view pointers live in webview `localStorage` where a storage wipe loses them and the
dead `SIDEBAR_EXPANDED_KEY` intent never landed. 0.0.17 integration and the 0.0.20 UX ship
both build on this slice.

## What Changes

- **Re-scope ADR-0034 into a new ADR** (supersedes 0034): Rust entity enums + `guard_*`
  methods stay the single source of truth; no `contracts/state-model.json`; no transient
  `*-ing` lifecycle states (unobservable under per-command sqlite transactions); sync
  status is TanStack Query's native pending/error/stale axis (`Conflicted`/`Rejected`
  dropped — file-merge Re-sync is moot under sqlite storage).
- **State-model mirror + contract test**: TS mirrors the entity state/transition/guard
  tables as typed constants; a contract test (in the style of `command_contract.rs`)
  proves the mirror matches the Rust source, failing on drift.
- **Client guard wiring**: UI reads the mirrored guard table through Store/Query to
  disable illegal actions (Default workspace / Unfiled project mutations, archived-entity
  edits) — advisory only; the orchestrator remains the enforcer.
- **Workspace-activity read-model**: new app-layer query deriving a per-workspace rollup
  of live surface runtime state (running / failed counts) server-side; a surface-status
  push event lets every window invalidate the matching Query key, so crashes and exits
  the user did not cause surface without a refetch cycle.
- **View pointers move to the orchestrator settings store**: `activeWorkspace`,
  `lastSession.<project>`, and `sidebar.expanded.<project>` become settings-store keys
  (global scope) read/written through Query/Store; `localStorage` persistence of
  `activeWorkspaceId` + `expandedProjectIds` in `apps/ui/app/lib/store.ts` is removed.
- Frozen seams honored: IPC command surface, wire protocol, and data model are extended
  additively only (new query + event; no schema migration — settings are file-backed).

## Capabilities

### New Capabilities

- `state-model-contract`: the Rust-authoritative entity state/transition/guard tables,
  their TS mirror, and the drift-proving contract test; advisory client guard semantics.
- `workspace-activity`: the server-derived per-workspace surface-activity rollup query
  and the surface-status push event that keeps every window's Query cache live.
- `view-pointers`: durable UI position pointers (`activeWorkspace`,
  `lastSession.<project>`, `sidebar.expanded.<project>`) in the orchestrator settings
  store, wired through Query/Store with optimistic local reads.

### Modified Capabilities

- `client-engine`: the "active-workspace selection persists through the browser storage
  layer" requirement changes — the active-workspace view pointer (and sidebar expand
  state) persist via the orchestrator settings store instead of `localStorage`; the
  Query-cache persistence requirement itself is unchanged.

## Impact

- `crates/orchestrator/src/app/` — new activity query (+ its view type); surface-status
  change event emission on the existing dispatch spine (ADR-0037/0041); state/guard
  tables exposed for the contract test.
- `crates/orchestrator/src/entities/` — no behavior change; guards/enums referenced as
  the contract source (session/surface gain guard coverage only if the mirror needs it).
- `apps/ui/app/lib/` — `store.ts` sheds persisted server-adjacent state; new
  `data/` query/mutation factories for view pointers + activity; guard mirror module;
  cross-window invalidation extended to the push event.
- `packages/client-bindings` — regenerated for the new query/event (generated surface,
  additive).
- `docs/adr/` — new ADR superseding ADR-0034; ADR-0034 marked superseded.
- Tests — Rust contract test; UI unit tests; desktop e2e for activity badge invalidation
  and view-pointer restore.
- `ROADMAP.md` — final 0.0.16 bullet checked on completion.
