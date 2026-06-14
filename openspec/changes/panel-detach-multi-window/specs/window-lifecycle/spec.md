## ADDED Requirements

### Requirement: Child window independence from parent lifecycle

Child windows created via panel detach or project-in-new-window SHALL remain open and functional even if the parent window is closed, restarted, or navigates to a different state.

#### Scenario: Closing parent window does not affect detached children

- **WHEN** user closes the main/parent window while detached child windows are open
- **THEN** the child windows remain open
- **AND** the child windows continue to display content and respond to user interactions
- **AND** closing the parent does not trigger any re-attach or cleanup logic in children

#### Scenario: Child can re-attach to restarted parent

- **WHEN** a child window is open and the parent window is restarted
- **THEN** the child window maintains its state and identity (placement/session)
- **AND** the child can still re-attach to the newly restarted parent
- **AND** re-attach succeeds by using the same (session, placement) pair

### Requirement: Window state tracking

The system SHALL track which windows are currently open and map placements to their owning windows, allowing the renderer to determine window identity from a query parameter.

#### Scenario: Child window knows its identity

- **WHEN** a child window is opened for a panel or project
- **THEN** the child window URL includes a query parameter indicating its type and context (e.g., `?w=detached&session=<id>&placement=<id>`)
- **AND** the renderer reads this parameter and displays the correct UI (e.g., showing a "Re-attach" button)

#### Scenario: Multiple children can be open simultaneously

- **WHEN** user detaches multiple panels from the same or different projects
- **THEN** each child window maintains its own identity
- **AND** each can be focused, interacted with, and re-attached independently
- **AND** closing one child does not affect other children

### Requirement: Window geometry persistence

Child window size, position, and state SHALL be persisted so that if a window is closed and recreated, it returns to a similar state.

#### Scenario: Child window geometry is remembered

- **WHEN** a user resizes or repositions a child window
- **THEN** the window geometry (size and position) is persisted
- **AND** if the child window is re-opened (e.g., re-detaching the same panel), it appears at the previously saved size and position

#### Scenario: Window state includes maximized/minimized state

- **WHEN** a user maximizes, minimizes, or restores a child window
- **THEN** the window state (maximized/restored) is saved
- **AND** the next time the window is opened, it appears in the same state
