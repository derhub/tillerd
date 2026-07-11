# ui-panel-compound

## ADDED Requirements

### Requirement: Panel title content

A panel leaf bound to a surface SHALL title itself with the session name, the surface
kind, and the elapsed time since the surface's PTY spawn (from the orchestrator-exposed
spawn timestamp), updating at a coarse interval.

#### Scenario: Title shows session, kind, and elapsed time

- **WHEN** a terminal surface has been running for over a minute
- **THEN** its panel header shows the session title, "terminal", and an elapsed-time
  indication

### Requirement: Toolbar buttons carry tooltips

Every icon-only button in a panel header toolbar (split horizontal, split vertical, detach, close) SHALL show a tooltip naming the action.

#### Scenario: Hovering a split button

- **WHEN** the user hovers the split-vertical button
- **THEN** a tooltip names the action

### Requirement: Close surface confirmation

Closing a surface-bound panel SHALL prompt a confirmation dialog stating that the surface
process will be terminated, with a "Don't ask again" option persisted via the settings
store. When the preference is set, close SHALL act immediately. Close SHALL hard-remove:
the launch-spec item is dropped and the PTY terminated.

#### Scenario: First close prompts

- **WHEN** the user activates close on a running terminal panel with no stored preference
- **THEN** a confirmation dialog appears and confirming terminates the PTY and removes
  the panel

#### Scenario: Don't-ask-again persists

- **WHEN** the user confirms with "Don't ask again" checked, restarts, and closes another
  surface
- **THEN** no dialog appears and the surface closes immediately

### Requirement: Panel lifecycle motion

Panel create and destroy SHALL animate opacity only (0→1 on create, 1→0 on destroy) using
the frozen motion tokens, with no layout shift; layout changes from add/remove SHALL fade
at the same cadence.

#### Scenario: New panel fades in

- **WHEN** a split creates a new leaf
- **THEN** the leaf fades in at the fast motion token with no neighboring panel jumping

### Requirement: Divider reset

Double-clicking a resize divider between panels SHALL reset the adjacent panels to an
equal split.

#### Scenario: Double-click resets

- **WHEN** two panels are unevenly sized and the user double-clicks their divider
- **THEN** both panels return to equal size

### Requirement: Empty panel picker

An empty panel leaf SHALL present a picker listing the available surface kinds (terminal
only in 0.x) and spawn the chosen kind into that leaf's placement.

#### Scenario: Picking terminal spawns into the leaf

- **WHEN** the user picks "terminal" in an empty leaf created by a split
- **THEN** a terminal surface spawns bound to that leaf's placement
