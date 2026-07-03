# Design: state-model-query-store

## Context

Final 0.0.16 item. ADR-0034 (proposed, deferred) named view pointers, guards, and a
workspace-activity read-model, but was written against `state.db` and file-merge sync —
both gone since ADR-0036. The in-force set today: ADR-0036 (entities own behavior, dumb
infra), ADR-0037/0041 (synchronous event dispatch on the tower bus spine), ADR-0038
(infra raw, app owns domain logic), ADR-0039/0040 (TanStack engine, file-based routing),
ADR-0042 (client-provided stream subscriptions). This design wires the three deferred
concerns through that reality; the accompanying ADR supersedes ADR-0034.

Current state (mined):

- Entities carry typed status enums (`WorkspaceStatus`/`ProjectStatus`/`SessionStatus`
  `{Active, Archived}`; `SurfaceStatus {Pending, Live, Failed, Idle}`) and guard methods
  (`guard_not_default`, `guard_active`, `guard_archived` on workspace;
  `guard_not_unfiled`, `guard_active`, `guard_archived` on project). No client mirror.
- No activity rollup query; the UI has no surface-status read at all.
- `activeWorkspaceId` + `expandedProjectIds` persist in `localStorage`
  (`apps/ui/app/lib/store.ts`); `SIDEBAR_EXPANDED_KEY` in `settings/keys.ts` is dead.
- Settings store is file-backed scoped KV (`infra/config/setting.rs`) with a full app
  API (`ApplySetting`, `GetSetting`, `ListSettings`, `ResolveSetting(s)`).
- Surface events flow as `DomainChannelEvent {Bytes, Status, Exit, Error}` keyed
  `surface://{id}` through `Registry::dispatch`; cross-window Query invalidation is a
  client-side broadcast with an ~80ms coalescing flush (`crossWindowSync.ts`).

## Goals / Non-Goals

**Goals**

- One drift-proof source of truth for entity states, transitions, and guards; client
  actions disabled from it.
- Per-workspace activity rollup queryable in one round trip and kept live by push.
- View pointers durable in the settings store, optimistic in the UI.

**Non-Goals**

- No transient `*-ing` lifecycle states, no sync-status enum, no conflict UI (superseded
  with ADR-0034's file-merge context).
- No schema migration — no new sqlite tables or columns.
- No new crates (crate-layout preference: modules in existing crates).
- No change to IPC wire protocol, ACL mechanism, or the frozen 0.0.6 seams (additive
  command/event surface only).
- Per-window restore-after-quit of URL intent stays deferred (unchanged from ADR-0034).

## Decisions

### D1 — Contract as a committed JSON fixture generated from Rust, asserted from both sides

The entity layer gains a `state_model()` module exporting the per-entity tables as static
data: states, legal transitions, and guard rules (`entity`, `action`, `rule` triples,
e.g. `workspace / delete / not_default`). A Rust test serializes the tables and asserts
they match a committed fixture (`state-model.contract.json`, next to the test — same
pattern as `command_contract.rs` / `generate_context!`). A TS test asserts the client
mirror (typed constants in `apps/ui/app/lib/stateModel.ts`) matches the same fixture.
Either side drifting fails its test; the fixture diff makes the change reviewable.

*Alternative — authored `contracts/state-model.json` loaded by both sides (ADR-0034):*
rejected by interview decision; guards live in Rust code under ADR-0036, so an authored
file would duplicate, not define. *Alternative — TS-only mirror with no test:* silent
divergence, the exact failure ADR-0034 names.

### D2 — Guard mirror evaluates on data the client already has

Guard rules reference only fields present in the client's existing read models
(`is_default`, `is_unfiled`, `status`). The mirror module exposes
`can(entity, action, row) -> boolean`; components call it for enablement — no new
queries, no per-component conditionals. Server remains the enforcer; a rejected command
surfaces through the existing `MutationCache.onError` → notification path.

*Alternative — server-computed `allowedActions` per row:* another payload on every list
read and a second source of truth to keep live; rejected.

### D3 — Activity rollup is one SQL aggregate over persisted surface status

`ListWorkspaceActivity` (app-layer Query, `app/workspace/`) returns
`{workspace_id, running, failed}` rows via one `GROUP BY` join
(workspace ← project ← session ← surface on `status`), reading the persisted
`SurfaceStatus` column the runtime already maintains. No domain field, no cache — derive
at query time (ADR-0034's rollup intent, re-based from in-memory `ProxyState` onto the
persisted status the sqlite world already has).

*Alternative — in-memory rollup from `ProxyState`:* `ProxyState` lives in the daemon
client (infra) and would leak infra into app; persisted status is already the
authoritative post-transition value. *Alternative — client-side compose from
`ListSurfacesBySession`:* whole-tree fan-out, banned by react-data-rules.

### D4 — Surface-status push rides the existing subscription spine

Where the app layer transitions a surface's status (spawn confirmed, exit, error), it
emits a `surface_status_changed {surface_id, session_id, workspace_id, status}` event on
the ADR-0037/0041 dispatch spine, exposed to windows over the ADR-0042 subscription
transport (a lifecycle-scoped subscription opened by each window, like the existing
notification feed). The client handler feeds the same coalescing flush
(`crossWindowSync`) to invalidate `["workspace-activity"]` and
`["surfaces", session_id]`. Push is additive — the rollup query stays correct without it;
push only bounds staleness.

*Alternative — reuse notification events as the invalidation trigger:* notifications are
user-facing signal with their own persistence/pruning; coupling cache coherence to them
inverts their purpose.

### D5 — View pointers are settings-store keys behind one Query key

Keys (global scope): `view.active-workspace` (workspace id),
`view.last-session.<project_id>` (session id), `sidebar.expanded.<project_id>`
(present = expanded — adopts the dead `SIDEBAR_EXPANDED_KEY` intent as a prefix).
Per-project keys, not one blob: concurrent windows toggling different projects must not
read-modify-write clobber each other. One `queryOptions` factory
(`["view-pointers"]`) fetches all three groups via `ListSettings`/`ResolveSettings` in a
single round trip; writes are `ApplySetting` mutations with optimistic `setQueryData` +
`meta.invalidates: [["view-pointers"]]` so sibling windows converge over the existing
broadcast. `uiStore` keeps the in-memory reactive copy for synchronous reads but drops
its `localStorage` persistence of `activeWorkspaceId`/`expandedProjectIds`; the persisted
Query cache covers cold-start hydration (client-engine delta).

*Alternative — keep localStorage:* lost on webview-storage wipe, invisible to other
future hosts, and leaves the settings-store intent dead. *Alternative — single JSON blob
key:* cross-window clobber.

### D6 — Lifecycle resolution at the single consumption point

Pointer resolution (`archived/deleted workspace → Default`, `stale lastSession →
ignored`) happens in the one place each pointer is consumed (workspace scope derivation,
project open), comparing against the already-cached entity lists — no server round trip.
A fallback rewrites the pointer once (fire-and-forget) so the stale value does not
re-resolve every start.

## Risks / Trade-offs

- [Spawn burst floods status events] → events enter the existing ~80ms coalescing flush;
  one invalidation pass per window per burst (spec scenario covers it).
- [Sidebar toggle spams settings writes] → optimistic UI is instant; the `ApplySetting`
  mutation itself is the debounce boundary (one write per toggle is acceptable — file-KV
  write is cheap; revisit only if profiling says otherwise).
- [Persisted `SurfaceStatus` lags true runtime state briefly] → acceptable: the push
  event fires after the status write commits, so invalidation always reads the
  post-transition row.
- [Contract fixture churn noise in review] → fixture is small, sorted, and diffs
  line-per-fact; churn is the feature (drift made visible).
- [Guard mirror grows stale semantics (rule exists but client lacks a field)] → contract
  test includes the rule's required fields; TS mirror test fails if the view type lacks
  them.

## Migration Plan

Clean cutover, pre-v1, no data migration:

1. Backend lands first (tables module, contract fixture + test, activity query, status
   event) — additive, no UI dependency.
2. Client bindings regenerate; UI lands mirror + guards, view-pointer queries, activity
   consumption; `localStorage` persistence for the two migrated fields is deleted (old
   values silently ignored — dev-only data).
3. ADR published; ADR-0034 marked superseded; ROADMAP bullet checked at archive.

Rollback = revert the branch; no persistent format changed.

## Open Questions

- ADR-0034 (proposed) must be recorded as superseded by the new ADR — handled by the adr
  step of this change.
- Whether `workspace_id` belongs on the status event payload vs a client-side
  session→workspace lookup from cached lists: payload chosen (server has it in one join;
  saves a cross-cache lookup) — flag for review, cheap to change.
