# state-model-contract

## Purpose

The entity state model — lifecycle states, legal transitions, and action guards — as a
single Rust-authoritative contract mirrored to the client, with a drift-proving test.
Re-scopes the deferred state-model contract onto the de-abstracted storage architecture:
no shared contract file, no transient `*-ing` lifecycle states, sync status expressed by
the server-state cache's native pending/error/stale axis.

## Requirements

### Requirement: Entity layer is the single source of state and guard truth

The entity layer SHALL define, per entity, the closed set of lifecycle states, the legal
transitions between them, and the guard rules that reject actions (Default-workspace and
Unfiled-project mutation guards, archived-entity edit guards, archive-before-hard-delete).
No transient in-progress lifecycle states SHALL be introduced: every mutation commits in a
single per-command transaction, so only stable states are observable. The entity layer
SHALL expose the state/transition/guard tables in a machine-readable form usable by a
verification test, without moving guard logic out of the entities.

#### Scenario: Guard rejection is decided by the entity layer

- **WHEN** a mutation command targets an entity a guard protects (e.g. delete the Default
  workspace, rename an archived project)
- **THEN** the command fails with a typed guard error derived from the entity rule, and no
  state is written

#### Scenario: Only stable states are observable

- **WHEN** any client reads entity state through a query at any point during or after a
  mutation
- **THEN** the state read is one of the entity's declared stable states, never a partial
  or in-progress value

### Requirement: Client mirrors the state and guard tables as typed constants

The client SHALL carry a typed mirror of each entity's states, transitions, and guard
rules as constants, and SHALL derive all action-enablement decisions (disabled menu items,
hidden actions) from that mirror — never from ad-hoc per-component conditions.

#### Scenario: Illegal action is disabled before dispatch

- **WHEN** the UI renders an action whose guard would reject the target entity (e.g.
  "Delete" on the Default workspace, "Rename" on an archived project)
- **THEN** the action renders disabled, derived from the mirrored guard table, and no
  command is dispatched on activation

### Requirement: A contract test proves the mirror matches the source

An automated contract test SHALL compare the client mirror against the entity layer's
machine-readable tables and fail on any divergence — a state, transition, or guard added,
removed, or renamed on one side only. The test SHALL run in the default verification gate.

#### Scenario: Drift fails the build

- **WHEN** an entity gains a new lifecycle state or guard and the client mirror is not
  updated in the same change
- **THEN** the contract test fails, naming the diverging entity and table

### Requirement: Client guards are advisory; the server enforces

Client-side guard evaluation SHALL be advisory only. The server SHALL enforce every guard
regardless of client behavior. A guard rejection SHALL be recorded by the orchestrator as
an error notification (the orchestrator is the sole notification recorder; the client
never records) and pushed to windows over the notification channel.

#### Scenario: Bypassing the client guard still fails safely

- **WHEN** a guarded command reaches the server despite the client mirror (stale window,
  race, or direct dispatch)
- **THEN** the server rejects it with the typed guard error, records a `command-error`
  notification server-side, and every window's notification feed receives it over the
  push channel
