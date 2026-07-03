# workspace-activity

## Purpose

A server-derived, per-workspace rollup of live surface runtime state, kept live in every
window by a surface-status push event — the read-model behind workspace/session activity
badges.

## Requirements

### Requirement: Server derives the workspace-activity rollup

The orchestrator SHALL expose a read query returning, per workspace, an activity rollup
derived from the runtime state of the surfaces under it (counts of running and failed
surfaces at minimum). The rollup SHALL be computed server-side from runtime surface
state at query time; it SHALL NOT be persisted as a domain field and SHALL NOT require
the client to enumerate sessions or surfaces to assemble it.

#### Scenario: Rollup reflects runtime surface state

- **WHEN** a workspace contains sessions whose surfaces are running, failed, or idle
- **THEN** the activity query returns that workspace's counts derived from the live
  runtime state, in one round trip for all workspaces

#### Scenario: Rollup is not a stored field

- **WHEN** the domain schema is inspected
- **THEN** no activity column or table exists; the rollup exists only as a query result

### Requirement: Surface runtime status changes push to every window

When a surface's runtime status changes (spawned, exited, crashed, closed), the
orchestrator SHALL emit a status-change event on the existing event-dispatch spine,
delivered to every connected window — including windows that did not cause the change.

#### Scenario: A crash the user did not cause is pushed

- **WHEN** a surface process crashes while the user performs no action
- **THEN** every connected window receives the surface-status event without polling

### Requirement: Windows invalidate the activity read-model on the push event

Each window SHALL, on receiving a surface-status event, invalidate its cached
workspace-activity query (and any surface-state query keyed to the affected surface) so
the next render reads fresh rollup data. Event bursts SHALL be coalesced so a burst costs
one invalidation pass per window, consistent with the existing cross-window invalidation
guard.

#### Scenario: Activity badge updates without user action

- **WHEN** a surface crashes in a session of a visible workspace
- **THEN** the workspace's activity badge reflects the failure in every open window
  without any user-initiated refetch

#### Scenario: A spawn burst coalesces

- **WHEN** a session launch spawns several surfaces in quick succession
- **THEN** each window performs a bounded number of invalidation passes (coalesced), not
  one per surface event
