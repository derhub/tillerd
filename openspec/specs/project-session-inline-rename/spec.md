# project-session-inline-rename Specification

## Purpose
TBD - created by archiving change workspace-management. Update Purpose after archive.
## Requirements
### Requirement: Double-click project name to rename inline

Users can edit a project name by double-clicking it in the sidebar. The name field becomes an editable text input. Enter confirms the change; Escape cancels without saving.

#### Scenario: Rename project by double-clicking and pressing Enter

- **WHEN** user double-clicks a project name in the sidebar
- **THEN** the project name becomes an editable text input field with cursor focus and current name selected

- **WHEN** user modifies the text and presses Enter
- **THEN** the project is renamed, the input closes, and the new name is displayed in the sidebar

#### Scenario: Cancel project rename by pressing Escape

- **WHEN** user double-clicks a project name and starts editing
- **THEN** text input is active

- **WHEN** user presses Escape before pressing Enter
- **THEN** the input closes without saving and the original name is restored in the sidebar

#### Scenario: Empty project name is rejected

- **WHEN** user edits a project name and clears all text
- **THEN** the input remains active and displays a visual indicator (e.g., red border or error text)

- **WHEN** user presses Enter with empty text
- **THEN** the rename is rejected and the original name is restored

### Requirement: Double-click session name to rename inline

Users can edit a session name by double-clicking it in the sidebar. The name field becomes an editable text input. Enter confirms the change; Escape cancels.

#### Scenario: Rename session by double-clicking and pressing Enter

- **WHEN** user double-clicks a session name in the sidebar
- **THEN** the session name becomes an editable text input field with cursor focus and current name selected

- **WHEN** user modifies the text and presses Enter
- **THEN** the session is renamed, the input closes, and the new name is displayed in the sidebar

#### Scenario: Cancel session rename by pressing Escape

- **WHEN** user double-clicks a session name and starts editing
- **THEN** text input is active

- **WHEN** user presses Escape before pressing Enter
- **THEN** the input closes without saving and the original name is restored in the sidebar

#### Scenario: Empty session name reverts to session ID prefix

- **WHEN** user edits a session name and clears all text
- **THEN** pressing Enter accepts the empty name

- **WHEN** the session name is empty and the view reloads
- **THEN** the session displays as the first 8 characters of its session ID (default fallback)
