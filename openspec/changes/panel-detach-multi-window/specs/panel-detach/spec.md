## ADDED Requirements

### Requirement: Detach button in panel header

The panel header SHALL contain a "Detach" button that, when clicked, tears off the panel into a new child window. The parent window SHALL display a greyed-out placeholder in the panel's original location with a "Focus →" button that brings the child window to the foreground.

#### Scenario: User detaches a terminal panel

- **WHEN** user clicks the detach button in a terminal panel header
- **THEN** a new child window opens containing that terminal panel
- **AND** the parent window shows a greyed placeholder where the panel was
- **AND** the placeholder displays a "Focus →" button

#### Scenario: User focuses child window from placeholder

- **WHEN** user clicks the "Focus →" button in the parent placeholder
- **THEN** the child window is brought to the foreground
- **AND** focus is transferred to the child window

#### Scenario: Detach button only appears on live panels

- **WHEN** a panel contains an empty surface (no content)
- **THEN** the detach button is not shown in the panel header

#### Scenario: Detach button is hidden when only one panel exists

- **WHEN** the parent window contains only one panel
- **THEN** the detach button is disabled or hidden
- **AND** a tooltip explains "Cannot detach the last panel"

### Requirement: Panel appears in correct context in child window

When a panel is detached into a child window, the panel SHALL maintain its session and surface binding and render the correct content (e.g., the terminal surface it was displaying).

#### Scenario: Detached terminal shows live session content

- **WHEN** a terminal panel displaying a live session is detached
- **THEN** the child window displays the terminal with the same session and PTY
- **AND** output from that session appears in real-time in both parent and child (if parent placeholder allows content)

#### Scenario: Child window inherits placement and session context

- **WHEN** a panel bound to a (session, placement) is detached
- **THEN** the child window knows its session and placement
- **AND** navigating to another session in the parent does not affect the child's session

### Requirement: Re-attach action in child window

A child window containing a detached panel SHALL provide a "Re-attach" action that returns the panel to its parent window and auto-focuses the parent.

#### Scenario: User re-attaches from child window

- **WHEN** user clicks "Re-attach" in the child window's toolbar or context menu
- **THEN** the panel returns to its original position in the parent window
- **AND** the parent window is brought to foreground and focused
- **AND** the child window closes

#### Scenario: Re-attach action is not available in main window

- **WHEN** a panel is in the main/parent window
- **THEN** the "Re-attach" action is not shown

### Requirement: Child window independence

A child window spawned by detaching a panel SHALL not be affected by the parent window's closure or session changes.

#### Scenario: Closing parent does not close child

- **WHEN** the parent window is closed
- **THEN** any detached child windows remain open
- **AND** the child window continues to display and update its content

#### Scenario: Child window can survive parent restart

- **WHEN** a panel has been detached and the parent window is restarted
- **THEN** the child window's placement and session are preserved
- **AND** the child can re-attach to a restarted parent using the same placement/session pair
