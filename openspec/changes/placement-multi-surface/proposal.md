## Why

A session's panels must hold their own surfaces (multiple terminals now, diffs later), switching sessions must never reveal another session's surfaces, and shared shell chrome (sidebar, status badge) must stay constant across sessions. Today the renderer points terminal panels at the route `<Outlet/>`, so a session has effectively one terminal and the panel tree -- not the launch spec -- looks like the owner of surfaces. The panel-surface seam freezes at 0.0.6, so this must ship now, not as a post-freeze additive item.

ADR-0030 records the decision: the launch spec is the single source of truth for a session's surfaces; the panel tree carries geometry only and binds a surface by `placement`. This change implements that seam.

## What Changes

- **BREAKING** `placement` becomes a unique, orchestrator-minted slot id (a UUID, unique per session) rather than the fixed `center`/`side` enum. A session holds N surfaces, each at its own placement; a launch template carries no placement, which is minted when an item enters a session spec. (Pre-v1; no migration owed beyond lazy null->placement.)
- A panel binds its surface by `placement` and never owns a surface id. The panel tree is geometry only (splits, sizes, tabs).
- Resume generalizes from terminal-only, one-per-session (`find_session_terminal_surface`) to keyed on `(session, placement)` for any surface kind and any count. All of a session's surfaces re-attach by placement on revisit.
- On session open, the panel tree reconciles against the launch spec (spec-authoritative): every spec placement gets a panel (its stored geometry, or a default); a panel whose placement is absent from the spec is dropped. Stored geometry is a best-effort hint that self-heals.
- Spawning a surface is initiated from an empty leaf's picker: the orchestrator appends a launch item, mints the placement, and creates the surface, and the acting leaf binds to it. Splitting a panel is pure geometry. Closing a surface is a hard remove: it drops the launch item and terminates the surface (distinct from session archive, which soft-deletes and preserves the pseudo-terminal). UI spawn/close diverges the session launch spec (ADR-0021 add/remove-items), not a UI-only act.
- The sidebar and host-status badge move out of the panel tree and render as chrome. The tree holds only session surfaces and empty geometry, so surfaces are always session-scoped and chrome is always shared -- no per-content scope flag.
- A fresh session starts with an empty launch spec and no auto-created surface; its default panel tree is the sidebar (chrome) plus a single empty leaf, from which the user spawns the first surface. This flips today's auto-open-terminal behavior.

Out of scope (open follow-ups, deferred): a PTY eviction policy for live pseudo-terminals that detach keeps alive across many sessions; diff-surface specifics (0.1.x per roadmap).

## Capabilities

### New Capabilities

<!-- none: every facet of this seam has an existing owner spec; one requirement, one home -->

### Modified Capabilities

- `launch-spec`: `placement` is a unique spec-minted slot id (supersedes the `center`/`side` enum); a launch item's placement is the durable binding to a panel.
- `launch-item`: placement is a required unique slot id that binds the produced surface to a panel, not a `center`/`side` hint.
- `ui-panel-model`: a panel binds a surface by `placement` and never owns a surface id; placement supersedes content-type as the binding key.
- `ui-shell`: the sidebar and status badge render as chrome outside the panel tree; the recursive tree binds surfaces by placement, and the sidebar is no longer a panel display mode.
- `layout-persistence`: on session open the panel tree reconciles against the launch spec -- every spec placement gets a panel, an orphan panel is dropped, and stored geometry self-heals.
- `surface-runtime`: resume is keyed on `(session, placement)`, generalizing reconnect from terminal-only/one-per-session to any surface kind and count.
- `workspace-persistence`: persist and resume N surfaces per session keyed by placement (supersedes the terminal-only, one-per-session surface row).
- `session-container`: adding or removing a surface is a launch-spec divergence keyed by placement; resume restores all of a session's surfaces by placement.

## Impact

- Affected specs: the eight modified capabilities above.
- Backend (Rust orchestrator): `find_session_terminal_surface` generalizes to `resume(session, placement)`; the product store persists N placement-keyed surfaces per session; add/remove-surface writes the session launch spec.
- UI (react-router SPA): panel leaves resolve a surface by placement instead of rendering `<Outlet/>`; session open reconciles the tree against the spec; spawn/close call the orchestrator to diverge the spec; sidebar and status badge lift out of the panel tree into the app shell.
- Design authority: ADR-0030 (amends ADR-0021 placement granularity); ADR-0024 (proxy-per-surface) unchanged -- only how a panel resolves to a surface and how many surfaces a session may hold changes.
- The 0.0.6 terminal-revisit slice already shipped is the first slice of this seam.
