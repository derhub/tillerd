# ui-entity-actions

## ADDED Requirements

### Requirement: Every exposed operation has a UI equivalent

Every app-layer operation exposed over the transport SHALL be reachable through at least
one UI affordance (context menu item, palette command, manager view control, or dialog).
The operation-to-affordance mapping SHALL cover: pin/unpin, archive/restore,
delete/discard, duplicate, move, rename, reorder, stop-surfaces, and search for the
entities that expose them (workspace, project, session, surface, command, template), plus
notification management and settings/profile/theme operations.

#### Scenario: Coverage holds for a workspace

- **WHEN** the user opens the workspace context menu
- **THEN** rename, pin/unpin, archive, delete, and stop-surfaces are offered

#### Scenario: Coverage holds for a session

- **WHEN** the user opens a session row context menu
- **THEN** rename, pin/unpin, archive, delete, duplicate, move to project, and
  stop-surfaces are offered

### Requirement: Pinned entities order first

Pinning a workspace, project, session, command, or template SHALL order it before
unpinned siblings in its list, with a visible pinned indication; unpinning restores
normal ordering.

#### Scenario: Pinning reorders the list

- **WHEN** the user pins a project below other projects
- **THEN** the project renders in the pinned group above unpinned projects

### Requirement: Archived entities are viewable and restorable

Each entity list with archive support SHALL offer an "archived" filter or section from
which an archived entity can be restored or permanently deleted. Restore SHALL return the
entity to its active list.

#### Scenario: Restoring an archived session

- **WHEN** the user opens the archived sessions section and restores a session
- **THEN** the session reappears in its project's active list

### Requirement: Move flows

Moving a project to another workspace and moving a session to another project SHALL be
offered via a picker (context menu submenu or dialog listing valid targets). The moved
entity SHALL appear under the target parent without restart.

#### Scenario: Moving a session

- **WHEN** the user moves a session to another project via the picker
- **THEN** the session lists under the target project and its surfaces keep running

### Requirement: Stop-surfaces with confirmation

Stop-surfaces on a workspace, project, or session SHALL terminate the scope's running
surfaces after a confirmation stating the scope, and SHALL reflect stopped status in the
affected rows.

#### Scenario: Stopping a project's surfaces

- **WHEN** the user invokes stop-surfaces on a project and confirms
- **THEN** every running surface under that project stops and session rows show stopped
  status

### Requirement: Search view

The Search activity-bar view SHALL search projects and sessions by name across the active
workspace, listing grouped results that navigate on activation. The existing session
search palette flow remains.

#### Scenario: Searching for a session

- **WHEN** the user types a query matching a session title in the Search view
- **THEN** the session lists under its project group and activating it navigates to the
  session
