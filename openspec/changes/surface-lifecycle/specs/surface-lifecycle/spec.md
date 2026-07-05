## ADDED Requirements

### Requirement: Closing a terminal pane resets it to the empty picker

Closing a pane whose leaf is bound to a terminal surface SHALL terminate that surface's
pseudo-terminal via the existing close operation and reset the leaf to an empty
(unbound) leaf that presents the surface picker, keeping the leaf in the layout at its
current geometry. The pane SHALL NOT be removed from the tree. This behavior SHALL apply
regardless of how many panes exist, including when it is the only pane.

#### Scenario: Closing a terminal pane shows the picker in place

- **WHEN** the user closes a pane bound to a terminal surface
- **THEN** the surface's pseudo-terminal is terminated
- **AND** the same leaf remains in the layout, now empty and showing the surface picker

#### Scenario: The only pane can be closed

- **WHEN** a session has exactly one pane bound to a terminal surface and the user closes it
- **THEN** the terminal is terminated and the single leaf becomes an empty picker
- **AND** the close is not blocked

### Requirement: Closing an empty pane removes it

Closing a pane whose leaf is empty (not bound to a surface) SHALL remove that leaf from
the tree and collapse its parent split, except when it is the only leaf in the session.

#### Scenario: Closing an empty pane in a split collapses it

- **WHEN** the user closes an empty pane that is one child of a split group
- **THEN** the leaf is removed and the split collapses so the sibling fills the space

#### Scenario: The sole empty pane offers no close control

- **WHEN** a session has exactly one leaf and it is empty
- **THEN** no close control is presented for that leaf

### Requirement: The tree always retains at least one pane

The panel tree SHALL never be reduced to zero leaves by a close action. Every close that
would otherwise remove the last leaf SHALL instead leave a single empty leaf presenting
the surface picker.

#### Scenario: Last remaining pane never disappears

- **WHEN** any close action would remove the final leaf of a session
- **THEN** the session retains one empty leaf showing the surface picker instead of an empty layout

### Requirement: Clean exit holds output with a restart bar

When a terminal surface's process exits cleanly, the pane SHALL keep its final scrollback
visible and present an inline exit bar reporting the exit code with a Restart action and a
New surface action. The pane SHALL NOT be reset or removed until the user acts. Selecting
Restart SHALL spawn a fresh terminal surface into the same pane, at the pane's existing
geometry, and rebind the pane to it. Selecting New surface SHALL reset the leaf to an empty
picker.

#### Scenario: Clean exit keeps output and shows the bar

- **WHEN** a terminal's process exits cleanly
- **THEN** the pane keeps its final output visible
- **AND** an inline bar shows the exit code with Restart and New surface actions

#### Scenario: Restart puts a fresh shell in the same pane

- **WHEN** the user selects Restart on an exited pane
- **THEN** a fresh terminal surface is spawned into the same pane at its existing geometry
- **AND** the pane shows a live shell without the pane being removed or moved

#### Scenario: New surface resets to the picker

- **WHEN** the user selects New surface on an exited pane
- **THEN** the leaf is reset to an empty picker

### Requirement: Unclean exit dismisses to the empty picker

When a terminal surface's process exits uncleanly or the surface errors, the pane SHALL
present the existing failure overlay with Resume and Dismiss controls. Dismiss SHALL reset
the leaf to an empty picker rather than leaving a dead pane.

#### Scenario: Dismiss on failure resets the pane

- **WHEN** a terminal fails and the user selects Dismiss on the failure overlay
- **THEN** the leaf is reset to an empty picker

### Requirement: Close confirmation only when a process is running

The close-confirmation dialog SHALL be presented only when the pane's surface still has a
live process. Closing a pane whose process has exited, or an empty pane, SHALL proceed
immediately with no confirmation. The existing "don't ask again" preference SHALL continue
to suppress the dialog for running processes when set.

#### Scenario: Running process prompts before termination

- **WHEN** the user closes a pane whose terminal process is still running and "don't ask again" is not set
- **THEN** a confirmation dialog is shown before the process is terminated

#### Scenario: Exited pane closes without a prompt

- **WHEN** the user closes a pane whose terminal process has already exited
- **THEN** the pane resets or removes with no confirmation dialog

#### Scenario: Empty pane closes without a prompt

- **WHEN** the user closes an empty pane
- **THEN** the pane is removed with no confirmation dialog
