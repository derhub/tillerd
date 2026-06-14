# project-session-delete Specification

## Purpose
TBD - created by archiving change workspace-management. Update Purpose after archive.
## Requirements
### Requirement: Delete project via context menu with confirmation

Users can hard-delete a project by right-clicking and selecting "Delete" from the context menu. A confirmation dialog warns of cascading deletion of sessions and surfaces.

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

Users can hard-delete a session by right-clicking and selecting "Delete" from the context menu. A confirmation dialog warns of surface termination.

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
