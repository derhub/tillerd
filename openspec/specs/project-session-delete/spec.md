# project-session-delete Specification

## Purpose
Hard-deleting projects and sessions from the sidebar context menu: a confirmation dialog warns of cascading removal (sessions and surfaces under a project, surfaces under a session, all PTYs terminated), the sidebar updates in real time on confirm, and navigation falls back to the home/empty state when the currently viewed project or session is deleted.
## Requirements
### Requirement: Delete project via context menu with confirmation

The sidebar SHALL let the user hard-delete a project from its row's context menu, behind a confirmation dialog that warns of the cascading deletion of the project's sessions and surfaces.

#### Scenario: Delete project after confirming dialog

- **WHEN** user right-clicks a project name and selects "Delete"
- **THEN** a confirmation dialog displays: "Delete <project-name>? This will permanently delete all sessions and surfaces in this project."

- **WHEN** user clicks "Delete" in the dialog
- **THEN** the project is permanently removed from the sidebar, all sessions under it are deleted, and all PTYs are terminated

#### Scenario: Cancel project deletion

- **WHEN** user right-clicks a project and selects "Delete"
- **THEN** confirmation dialog appears

- **WHEN** user clicks "Cancel" or presses Escape
- **THEN** the dialog closes and the project remains in the sidebar unchanged

#### Scenario: Deleted project and its sessions vanish immediately

- **WHEN** user deletes a project
- **THEN** the project and all its sessions are removed from the sidebar in real-time without requiring a page reload

#### Scenario: Navigating away from a deleted project

- **WHEN** user is viewing a session in a project that is then deleted
- **THEN** after deletion, the app navigates to the home / empty state

### Requirement: Delete session via context menu with confirmation

The sidebar SHALL let the user hard-delete a session from its row's context menu, behind a confirmation dialog that warns that the session's surfaces terminate.

#### Scenario: Delete session after confirming dialog

- **WHEN** user right-clicks a session name and selects "Delete"
- **THEN** a confirmation dialog displays: "Delete <session-name>? This will permanently delete the session and terminate its PTYs."

- **WHEN** user clicks "Delete" in the dialog
- **THEN** the session is permanently removed from the sidebar, all surfaces within it are deleted, and all PTYs are terminated

#### Scenario: Cancel session deletion

- **WHEN** user right-clicks a session and selects "Delete"
- **THEN** confirmation dialog appears

- **WHEN** user clicks "Cancel" or presses Escape
- **THEN** the dialog closes and the session remains in the sidebar unchanged

#### Scenario: Deleted session vanishes immediately

- **WHEN** user deletes a session
- **THEN** the session is removed from the sidebar in real-time without requiring a page reload

#### Scenario: Navigating away from a deleted session

- **WHEN** user is viewing a session that is then deleted
- **THEN** after deletion, the app navigates to the home / empty state
