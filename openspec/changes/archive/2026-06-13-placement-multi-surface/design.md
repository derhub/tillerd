## Context

The architectural decision for this change is recorded in `docs/adr/0030-panels-bind-surfaces-by-placement.md` (amends ADR-0021 placement granularity); the workspace glossary is `CONTEXT.md`. This design covers HOW to land that decision across the orchestrator and the renderer. Read the ADR for the WHY; it is not repeated here.

Current state: the launch spec (ADR-0021, `spec_json`) and the panel tree (`session_layout`) both describe "what is in a session's view". The renderer points terminal panels at the route `<Outlet/>`, so a session has effectively one terminal and the panel tree -- not the spec -- looks like the owner of surfaces. The 0.0.6 terminal-revisit slice (`find_session_terminal_surface`, one terminal per session) is the first slice of the general seam this change completes. The panel-surface seam freezes at 0.0.6, so this is a must-ship, not a post-freeze additive item.

Constraints: pre-v1, breaking changes allowed; ADR-0024 (proxy-per-surface) and the daemon wire protocol are unchanged. Backend is the Rust orchestrator (ADR-0022/0023); the renderer is the react-router SPA. The embedded planning `context` block describing an SDK/engine split is stale and does not govern this change.

## Goals / Non-Goals

**Goals:**

- Make the launch spec the single source of truth for which surfaces a session has; the panel tree carries geometry plus a placement binding per leaf and never owns a surface id.
- Make `placement` a unique, spec-minted slot id so a session holds N surfaces; bind each panel to a surface by `(session, placement)`.
- Generalize resume from terminal-only/one-per-session to all of a session's surfaces, keyed on `(session, placement)`.
- Reconcile the panel tree against the launch spec on session open (spec-authoritative self-heal): every spec placement gets a panel; orphan panels are dropped.
- Make UI spawn/close diverge the session launch spec (append/remove a launch item), not a UI-only act.
- Lift the sidebar and host-status badge out of the panel tree into app-shell chrome.

**Non-Goals:**

- A PTY eviction policy for live pseudo-terminals that detach keeps alive across many sessions (open follow-up in ADR-0030).
- Diff-surface specifics (roadmap 0.1.x); this change makes the seam kind-agnostic but adds no diff surface.
- Changing the daemon wire protocol, proxy-per-surface lifecycle (ADR-0024), or the two-level id model (ADR-0023).
- A per-content scope registry (shared vs session); the surface-versus-chrome split is structural, so no scope flag exists.

## Decisions

### The launch spec owns surfaces; the panel tree binds by placement

Per ADR-0030: the spec is authoritative for which surfaces exist; the tree is geometry plus a leaf->placement binding. A leaf resolves its surface by `(session, placement)` and never stores a surface id.

_Alternative considered (rejected in ADR-0030):_ each panel leaf carries a `surfaceId`, splitting a panel creates a surface, and a `CONTENT_SCOPE` registry marks each content type shared or session-scoped. Rejected: it makes the panel tree a second source of truth for surfaces and conflicts with ADR-0021. The surface-vs-chrome split makes a scope registry unnecessary -- the tree holds only session surfaces, chrome lives outside it.

### `placement` is an orchestrator-minted UUID, unique per session

Supersedes ADR-0021's `center`/`side`. Placement is a UUID the orchestrator mints when a launch item enters a session spec -- template instantiation, or a later spawn. It is session-scoped uniqueness only; global per-instance uniqueness is already `surface_id`'s job. A launch template carries no placement (a UUID cannot be pre-authored). Placements are never reused: each spawn mints a fresh one; a closed placement is retired. Uniqueness is enforced at spec validation, at surface creation, and at the persistence row (`(session, placement)` unique among live surfaces).

_Alternatives considered:_ a reserved human-named default (e.g. `"main"`) plus authored named slots in templates -- rejected to keep one minting path and avoid a second authoring surface; a per-placement UUID is opaque but uniform. Keeping `placement` nullable with null as the default slot -- rejected because SQLite treats NULLs as distinct, so the `UNIQUE(session, placement)` constraint would not enforce one default per session.

### Resume is keyed on `(session, placement)`

`find_session_terminal_surface` generalizes to a `(session, placement)` lookup over any surface kind and count. The existing reconnect-by-surface-identifier requirement (surface-runtime) is unchanged for the proxy re-attach; the new lookup is how the UI picks which surface re-attaches to which panel. On orchestrator startup, every non-archived surface reconnects and is exposed with its placement.

### Consistency is spec-authoritative; geometry self-heals

On session open the UI reconciles stored geometry against the spec's placements: every spec placement gets a panel (stored geometry or a default leaf appended to the root); a leaf bound to a placement absent from the spec is dropped; an empty (unbound) leaf is kept as durable geometry. Stored geometry is a best-effort hint, so a stale or corrupt layout cannot show or hide a surface that disagrees with the spec, while user-arranged empty panels survive.

### Spawn from an empty leaf; close is a hard remove

Splitting a panel is pure geometry (it makes an empty leaf). The empty leaf's picker spawns: the orchestrator appends a launch item, mints the placement, creates the surface, and the acting leaf binds to the returned placement. Reconciliation is the fallback that adds a default leaf for any spec placement with no leaf (restart, migration, another client). Closing a surface is a hard remove -- it drops the launch item and terminates the pseudo-terminal -- distinct from session archive, which soft-deletes and preserves the pseudo-terminal for restore; a closed surface is not resumed. Both route UI actions through the orchestrator's add/remove-surface, extended to write the spec.

### Sidebar and status badge are chrome; a fresh session is empty

The sidebar and status badge render in the app shell outside the panel tree. The `displayMode: 'sidebar'` panel mode and the sidebar entry in the default layout are removed; the session list renders as chrome. This guarantees chrome is always shared and surfaces always session-scoped, with no per-content flag. A fresh session has an empty launch spec and no auto-created surface; its default panel tree is the sidebar (chrome) plus a single empty leaf, from which the user spawns the first surface. The diff surface kind is deferred (roadmap 0.1.x).

## Risks / Trade-offs

- [Stored layouts from before this change contain a sidebar panel and `center`/`side` placements] -> Reconciliation against the spec drops orphan panels and the sidebar-mode group on load; lazy migration handles `spec_json`. Pre-v1, no long-term compatibility owed.
- [Live PTYs accumulate as detach keeps them alive across many sessions and placements] -> Out of scope here; tracked as the ADR-0030 eviction follow-up. `is_reachable`/detach semantics are unchanged meanwhile.
- [`(session, placement)` uniqueness must hold at three layers (spec validation, surface creation, persistence row)] -> A typed conflict error at each layer; the persistence row is the backstop. Tests assert all three.
- [Three requirement headers keep stale names after their bodies generalize: launch-item "Placement hint on surface creation", surface-runtime "Placement hint accepted at surface creation", workspace-persistence "Terminal surface row persistence and resume", ui-panel-model "Panel content type assignment"] -> Headers kept verbatim so OpenSpec archive matches by header; rename them in a follow-up cleanup. Listed in Open Questions.

## Migration Plan

1. Backend: generalize `find_session_terminal_surface` to a `(session, placement)` resolver; enforce `(session, placement)` uniqueness at surface creation and at the persistence row; extend add/remove-surface to append/remove launch items.
2. Persistence: allow N surface rows per session keyed by placement; lazy-migrate the existing null-placement terminal row to a minted placement on next open.
3. Renderer: replace `<Outlet/>`-bound terminal panels with leaves that resolve a surface by `(session, placement)`; reconcile the tree against the spec on open; route spawn/close through orchestrator add/remove-surface; lift the sidebar and status badge into chrome.
4. Rollback: pre-v1, no staged rollout; revert the branch. Stored layouts self-heal against the spec on next open regardless of direction.

## Open Questions

- PTY eviction policy for long-lived detached pseudo-terminals (ADR-0030 follow-up).
- Diff-surface specifics (roadmap 0.1.x) -- this change leaves the seam kind-agnostic.
- Follow-up rename of the four stale requirement headers once this change archives.
