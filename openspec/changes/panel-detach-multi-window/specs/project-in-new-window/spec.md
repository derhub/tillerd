## ADDED Requirements

### Requirement: Right-click context menu on sidebar project

A right-click on a project entry in the sidebar SHALL display a context menu with an "Open in new window" option. Selecting this option SHALL open the project's view (including its sessions and panels) in a new child window.

#### Scenario: User right-clicks a project

- **WHEN** user right-clicks on a project name in the sidebar
- **THEN** a context menu appears with options including "Open in new window"

#### Scenario: User opens project in new window

- **WHEN** user selects "Open in new window" from the project context menu
- **THEN** a new child window opens displaying that project's sessions and panels
- **AND** the parent sidebar marks the project with a pending-detach indicator (e.g., a small badge or visual cue)

#### Scenario: Parent sidebar shows pending-detach indicator

- **WHEN** a project has been opened in a new window
- **THEN** the project entry in the parent sidebar displays a pending-detach badge or indicator
- **AND** hovering over or clicking the indicator shows a "Focus →" button to bring the child window to front

### Requirement: Project in new window maintains session context

When a project is opened in a new window, all sessions and surfaces within that project SHALL render correctly and respond to user interactions as if the project were displayed in the main window.

#### Scenario: Sessions and terminals display in project window

- **WHEN** a project containing multiple sessions with terminals is opened in a new window
- **THEN** all sessions and their terminals appear in the child window
- **AND** user can interact with any terminal, create new sessions, etc. within the child window

#### Scenario: Child project window is independent of parent session selection

- **WHEN** a project is displayed in a child window
- **THEN** selecting a different project or session in the parent window does not change the child window's display
- **AND** the child window remains focused on its project

### Requirement: Re-attach or close project window

A child window showing a project SHALL allow the user to either re-attach the project back to the parent sidebar or close the child window independently.

#### Scenario: User re-attaches project to parent

- **WHEN** user performs a "Re-attach" action in the child project window (via toolbar or context menu)
- **THEN** the project view returns to the parent sidebar
- **AND** the pending-detach indicator is removed
- **AND** the parent window is brought to foreground and focused
- **AND** the child window closes

#### Scenario: User closes project window without re-attaching

- **WHEN** user closes the child project window (via window close button or OS)
- **THEN** the project remains visible in the parent sidebar with no pending-detach indicator
- **AND** the parent window is unaffected
