## ADDED Requirements

### Requirement: Active pane focus tracking and ring

The panel area SHALL track exactly one focused leaf at a time. The focused leaf SHALL be
set when the user interacts with a pane (pointer down within it, or navigating to it by
keyboard) and SHALL be shown with a visible focus ring distinct from the drop-target
indicator. When the focused leaf is removed or reset, focus SHALL move to a remaining leaf
so a focused leaf always exists while the session has any pane.

#### Scenario: Interacting with a pane focuses it

- **WHEN** the user presses the pointer down inside a pane
- **THEN** that pane becomes the focused leaf and shows the focus ring
- **AND** no other pane shows the focus ring

#### Scenario: Focus survives removal of the focused pane

- **WHEN** the focused pane is closed and removed
- **THEN** focus moves to a remaining pane

### Requirement: Directional keyboard pane navigation

The panel area SHALL provide directional navigation that moves focus from the focused leaf
to the nearest leaf in a given direction (left, right, up, down) based on pane geometry.
The default binding SHALL be `Cmd+Alt+Arrow`. Navigation SHALL be a no-op when no leaf
exists in the requested direction.

#### Scenario: Navigate to an adjacent pane

- **WHEN** two panes are split side by side, the left is focused, and the user presses the navigate-right binding
- **THEN** focus moves to the right pane

#### Scenario: No pane in the direction is a no-op

- **WHEN** the focused pane has no neighbor in the requested direction
- **THEN** focus does not change

### Requirement: Zoom a pane to fill the panel area

The panel area SHALL provide a toggle that expands the focused leaf to fill the entire
panel area, hiding its siblings, and restores the prior split layout when toggled again.
The default binding SHALL be `Cmd+Alt+Z`. Zoom SHALL be a transient view state and SHALL
NOT mutate or persist the split tree; a reload SHALL restore the un-zoomed layout.

#### Scenario: Zoom fills the area

- **WHEN** a pane in a split is focused and the user activates the zoom toggle
- **THEN** that pane fills the whole panel area and its siblings are hidden

#### Scenario: Zoom toggles back to the split

- **WHEN** a pane is zoomed and the user activates the zoom toggle again
- **THEN** the prior split layout is restored unchanged

#### Scenario: Zoom does not persist

- **WHEN** a pane is zoomed and the session is reloaded
- **THEN** the layout is restored un-zoomed from the persisted tree

### Requirement: Pane keybindings fire while a terminal is focused

Default keyboard shortcuts SHALL exist for split-right, split-down, close-surface, and
new-surface, and they SHALL act on the focused leaf. Because global shortcuts are
suppressed while a terminal holds keyboard focus, these pane bindings SHALL be dispatched
through the terminal's own key handler so they fire while a pane is focused, and SHALL be
prevented from also being written to the terminal as input.

#### Scenario: Split fires while the terminal is focused

- **WHEN** a terminal pane holds keyboard focus and the user presses the split-right binding
- **THEN** the focused pane splits to the right
- **AND** the binding's keystroke is not written into the terminal

#### Scenario: New surface fires while the terminal is focused

- **WHEN** a terminal pane holds keyboard focus and the user presses the new-surface binding
- **THEN** a new empty pane is created per the split/spawn rules
- **AND** the binding's keystroke is not written into the terminal
