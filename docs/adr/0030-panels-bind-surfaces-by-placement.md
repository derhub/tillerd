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
