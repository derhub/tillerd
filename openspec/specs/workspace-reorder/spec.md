# workspace-reorder Specification

## Purpose
TBD - created by archiving change workspace-management. Update Purpose after archive.
## Requirements
### Requirement: Drag-to-reorder projects in sidebar

Users can drag projects in the sidebar to reorder them. The order persists across app restarts.

#### Scenario: Drag project to new position within the sidebar

- **WHEN** user clicks and holds a project name and drags it to a new position in the project list
- **THEN** the project visually moves to the new position and a drop target indicator appears

- **WHEN** user releases the mouse
- **THEN** the project remains in the new position, the list reflows, and the backend records the new sort order

#### Scenario: Project order persists after app restart

- **WHEN** user reorders projects and closes the app
- **THEN** the next time the app launches, projects appear in the same order

#### Scenario: Drag project between Unfiled and named projects

- **WHEN** user drags a named project below the Unfiled group
- **THEN** the drag fails or the project cannot be moved past Unfiled (Unfiled always remains last)

### Requirement: Drag-to-reorder sessions within a project

Users can drag sessions within their parent project to reorder them. The order persists across app restarts.

#### Scenario: Drag session to new position within the project

- **WHEN** user clicks and holds a session name and drags it to a new position within its project
- **THEN** the session visually moves to the new position and a drop target indicator appears

- **WHEN** user releases the mouse
- **THEN** the session remains in the new position and the backend records the new sort order

#### Scenario: Session order persists after app restart

- **WHEN** user reorders sessions within a project and closes the app
- **THEN** the next time the app launches, sessions appear in the same order within their project

#### Scenario: Cannot drag session across projects

- **WHEN** user attempts to drag a session from one project to another project
- **THEN** the drag fails and the session remains in its original project

#### Scenario: New session appears at the end of the list

- **WHEN** user creates a new session in a project
- **THEN** the session appears at the bottom of the project's session list (highest sort order)
