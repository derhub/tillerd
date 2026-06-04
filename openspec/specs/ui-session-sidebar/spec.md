# ui-session-sidebar

## Purpose

Defines the session sidebar that lists active sessions, navigates to a session's terminal view, and provides a control to start a new session.

## Requirements

### Requirement: Session list display

The sidebar SHALL display the list of active sessions fetched from the server. Each row SHALL show the session ID (truncated to 8 characters) and the session's working directory basename. The list SHALL be fetched on mount and refreshed when the user navigates to a new session.

#### Scenario: Sessions load on mount

- **WHEN** the sidebar mounts
- **THEN** it fetches the session list and renders one row per active session

#### Scenario: Empty state

- **WHEN** there are no active sessions
- **THEN** the sidebar shows a message indicating no sessions are running

#### Scenario: Session row shows identity

- **WHEN** a session row is rendered
- **THEN** it displays a truncated session ID and the working directory name

### Requirement: Session navigation

Clicking a session row SHALL navigate the content area to the session's terminal view. The active session row SHALL be visually distinguished from inactive rows.

#### Scenario: Click navigates

- **WHEN** the user clicks a session row
- **THEN** the router navigates to the session's route and the terminal pane loads for that session

#### Scenario: Active row highlighted

- **WHEN** the current route corresponds to a session
- **THEN** that session's sidebar row is highlighted as active

### Requirement: New session action

The sidebar SHALL provide a control to start a new session. Activating it SHALL navigate to the shell content area in a state that opens a new session connection.

#### Scenario: New session button visible

- **WHEN** the sidebar is rendered
- **THEN** a "New session" button or equivalent control is visible

#### Scenario: New session navigates

- **WHEN** the user activates the new session control
- **THEN** the router navigates to a route that establishes a new session terminal
