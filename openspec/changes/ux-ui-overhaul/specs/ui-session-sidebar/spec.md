# ui-session-sidebar

## MODIFIED Requirements

### Requirement: Session list display

The Sessions activity-bar view SHALL display the list of active sessions grouped by
project, scoped to the active workspace. Each project group SHALL show the project name
as an expandable/collapsible heading. Sessions within a group SHALL show the inferred or
custom title (not truncated id or cwd basename) and a status badge reflecting the
session's surface state (starting / running / failed / idle). The Unfiled project group
SHALL be shown last when it contains sessions; it SHALL be hidden when it has no active
sessions. Session reads SHALL stay scoped by project and lazily paginated on expand. The
list SHALL be fetched on mount and refreshed when the user navigates to a new session or
when a session is created or archived.

#### Scenario: Sessions load on mount

- **WHEN** the sidebar mounts
- **THEN** it fetches the project and session lists and renders sessions grouped under
  their project

#### Scenario: Session row shows inferred title

- **WHEN** a session row is rendered and the session has a non-empty inferred or custom
  title
- **THEN** the row displays that title

#### Scenario: Session row shows a status badge

- **WHEN** a session has a running surface
- **THEN** its row shows the running status badge, and the badge updates when the surface
  fails or stops

#### Scenario: Unfiled group hidden when empty

- **WHEN** no active sessions belong to the Unfiled project
- **THEN** the Unfiled group heading is not rendered

#### Scenario: Unfiled group shown last when populated

- **WHEN** at least one active session belongs to the Unfiled project and at least one
  belongs to a named project
- **THEN** the Unfiled group is rendered after all named project groups

## ADDED Requirements

### Requirement: Project expand state persists

Each project group's expanded/collapsed state SHALL persist across restarts via the
settings store and restore on launch, defaulting to expanded.

#### Scenario: Collapse survives restart

- **WHEN** the user collapses a project group and restarts the application
- **THEN** that group renders collapsed on next launch

### Requirement: Zero state

When no projects exist in the active workspace, the sidebar SHALL render an empty state
and the panel area SHALL show a create-project call-to-action that opens the new-project
flow.

#### Scenario: First run shows the call-to-action

- **WHEN** the application opens a workspace containing no projects
- **THEN** the panel area shows a create-project call-to-action and activating it opens
  the new-project flow
