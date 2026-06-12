# ui-session-sidebar

## Purpose

Defines the session sidebar that lists active sessions, navigates to a session's terminal view, and provides a control to start a new session.
## Requirements
### Requirement: Session list display

The sidebar SHALL display the list of active sessions grouped by project. Each project group SHALL show the project name as a heading. Sessions within a group SHALL show the inferred or custom title (not truncated id or cwd basename). The Unfiled project group SHALL be shown last when it contains sessions; it SHALL be hidden when it has no active sessions. The list SHALL be fetched on mount and refreshed when the user navigates to a new session or when a session is created or archived.

#### Scenario: Sessions load on mount

- **WHEN** the sidebar mounts
- **THEN** it fetches the project and session lists and renders sessions grouped under their project

#### Scenario: Empty state

- **WHEN** there are no active sessions
- **THEN** the sidebar shows a message indicating no sessions are present

#### Scenario: Session row shows inferred title

- **WHEN** a session row is rendered and the session has a non-empty inferred or custom title
- **THEN** the row displays that title

#### Scenario: Unfiled group hidden when empty

- **WHEN** no active sessions belong to the Unfiled project
- **THEN** the Unfiled group heading is not rendered

#### Scenario: Unfiled group shown last when populated

- **WHEN** at least one active session belongs to the Unfiled project and at least one belongs to a named project
- **THEN** the Unfiled group is rendered after all named project groups

### Requirement: Session navigation

Clicking a session row SHALL navigate the content area to the session's terminal view. The active session row SHALL be visually distinguished from inactive rows.

#### Scenario: Click navigates

- **WHEN** the user clicks a session row
- **THEN** the router navigates to the session's route and the terminal pane loads for that session

#### Scenario: Active row highlighted

- **WHEN** the current route corresponds to a session
- **THEN** that session's sidebar row is highlighted as active

### Requirement: Project and session create actions

The sidebar SHALL provide controls to create a new project and to create a new session under a selected project. Activating the new-project control SHALL open a form or prompt that accepts a source kind and an optional name. Activating the new-session control for a project SHALL create a session under that project and navigate to it.

#### Scenario: New-project control visible

- **WHEN** the sidebar is rendered
- **THEN** a control to create a new project is visible

#### Scenario: New-session control visible per project

- **WHEN** a project group is rendered in the sidebar
- **THEN** a control to add a new session under that project is visible within or adjacent to the group

#### Scenario: New session navigates to session route

- **WHEN** the user activates the new-session control for a project
- **THEN** the orchestrator creates a session under that project, the router navigates to the session route, and the new session row appears in the sidebar

### Requirement: Session archive action

The sidebar SHALL provide a control per session row to archive that session. Activating it SHALL send an archive-session request to the orchestrator, remove the session row from the active list, and navigate away from the session route if that session was active.

#### Scenario: Archive control visible on session row

- **WHEN** a session row is rendered
- **THEN** an archive control (e.g., a context menu item or button) is accessible from that row

#### Scenario: Archive removes row and navigates away

- **WHEN** the user activates the archive control for the currently active session
- **THEN** the sidebar sends an archive-session request, removes the row from the list, and navigates to a neutral route

