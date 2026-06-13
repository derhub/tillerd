# 0030. Panels bind surfaces by placement; the launch spec owns surfaces

- Status: proposed
- Date: 2026-06-13
- Amends: ADR-0021 (placement granularity)

## Context

0.0.6 wired terminal revisit by re-attaching a session's terminal surface
(`find_session_terminal_surface`, one terminal per session). The requirement is
broader: each session's panels hold their own unique content (multiple terminals,
later diffs), switching sessions must never show another session's surfaces, and
shared shell chrome (the sidebar) stays the same across sessions.

Two per-session structures described "what is in a session's view": the launch spec
(ADR-0021, `spec_json`) and the panel tree (`session_layout`). The renderer pointed
terminal panels at the route `<Outlet/>`, so a session had effectively one terminal
and the panel tree, not the spec, looked like the owner of surfaces. ADR-0021
already declares surfaces (`items[] = {target, placement, command}`) and reserves
`placement` as the seam to a panel region, but ships only a fixed `center/side` set.

A proposed alternative had each panel leaf carry a `surfaceId` and splitting a panel
create a surface, plus a `CONTENT_SCOPE` registry marking each content type shared
or session-scoped. That makes the panel tree a second source of truth for surfaces
and conflicts with ADR-0021.

## Decision

- The launch spec is the single source of truth for which surfaces a session has.
  The panel tree carries geometry only (splits, sizes, tabs) and binds to a surface
  by `placement`, never by owning a surface id.
- `placement` is a unique, spec-minted slot id, not a fixed enum: a session holds N
  surfaces. This supersedes ADR-0021's minimal `center/side` placement.
- Resume is keyed on `(session, placement)`, generalizing
  `find_session_terminal_surface` to any surface kind and any count.
- The sidebar and status badge are chrome, rendered outside the panel tree. The tree
  holds only session surfaces, so surfaces are always session-scoped and chrome is
  always shared. There is no per-content scope flag.
- Consistency is spec-authoritative. On load, every spec placement gets a panel (its
  stored geometry, or a default); a panel whose placement is absent from the spec is
  dropped. Spawn appends a spec item and creates the surface; close removes the spec
  item and the surface; geometry is a best-effort hint that self-heals.

## Consequences

- Spawning a terminal is a session diverging its launch spec (ADR-0021's
  add/remove/edit-items clause), not a UI-only act; the surface is placement-bound.
- A scope registry (shared vs session per content type) is unnecessary; the
  surface-versus-chrome split is structural.
- All of a session's surfaces resume by placement on revisit; the terminal-only,
  one-per-session resume from 0.0.6 is the first slice of this.
- Surface lifecycle (ADR-0024, proxy-per-surface) is unchanged; this changes only
  how a panel resolves to a surface and how many surfaces a session may hold.
- Open follow-ups: placement is durable because it lives in the spec, but an
  eviction policy is still needed for the live PTYs that detach keeps alive across
  many sessions.

## Refinements (resolved 2026-06-13)

The seam above was resolved into these specifics before implementation:

- Placement is an orchestrator-minted UUID, unique per session. It is not
  human-named or hand-authored; `surface_id` remains the global per-instance id and
  placement is the per-session slot label. A UUID is session-scoped uniqueness only.
- A launch template carries no placement. The orchestrator mints a placement per
  launch item when the item enters a session spec (template instantiation, or a later
  spawn). The pre-existing single-terminal row (placement null) lazy-migrates to a
  minted placement on next open.
- Placements are never reused: each spawn mints a fresh UUID; a closed placement is
  retired. `(session, placement)` is unique among a session's live surfaces.
- Spawn: splitting a panel is pure geometry (it makes an empty leaf). The empty
  leaf's picker spawns -- the orchestrator appends a launch item, mints the placement,
  creates the surface, and the acting leaf binds to the returned placement.
  Reconciliation is the fallback that adds a default leaf for any spec placement with
  no leaf (restart, migration, another client).
- Close is a hard remove: it drops the spec item and terminates the PTY. This is
  distinct from session archive (soft-delete, PTY preserved for restore). A closed
  surface is not resumed.
- Reconciliation on session open: a leaf bound to a placement absent from the spec is
  dropped; an empty (unbound) leaf is kept as durable geometry; every spec placement
  lacking a leaf gets a default leaf.
- Default session: a fresh session has an empty launch spec and no auto-created
  surface. The default panel tree is the sidebar (chrome) plus a single empty leaf;
  the user spawns the first terminal from the empty leaf. The diff surface kind is
  deferred (roadmap 0.1.x).
