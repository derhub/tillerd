# sidebar-context-actions Specification

## Purpose
Right-click context menus on sidebar project and session rows: project rows expose Rename, Archive, Delete, and Open in new window; session rows expose Rename, Archive, and Delete. The menu opens at the cursor, closes on outside click or Escape, and is keyboard-navigable via Tab/Shift+Tab and Enter.
## Requirements
### Requirement: Right-click project row to open context menu with full action set

Right-clicking a project row in the sidebar SHALL open a context menu offering Rename, Archive, Delete, and Open in new window.

#### Scenario: Right-click project opens context menu

- **WHEN** user right-clicks a project name in the sidebar
- **THEN** a context menu appears at the cursor position with the following items: "Rename", "Archive", "Delete", "Open in new window"

#### Scenario: Rename action from context menu

- **WHEN** user right-clicks a project and clicks "Rename"
- **THEN** the project name enters inline-edit mode (double-click behavior)

#### Scenario: Archive action from context menu

- **WHEN** user right-clicks a project and clicks "Archive"
- **THEN** the project is soft-deleted and moves to an archived state (visible in a separate "Archived Projects" section or via a list filter)

#### Scenario: Delete action from context menu

- **WHEN** user right-clicks a project and clicks "Delete"
- **THEN** the delete confirmation dialog appears

#### Scenario: Open in new window action from context menu

- **WHEN** user right-clicks a project and clicks "Open in new window"
- **THEN** a new window opens displaying that project with its sessions, and the parent sidebar shows a pending-detach indicator on the project

#### Scenario: Context menu closes on outside click

- **WHEN** user opens the context menu
- **THEN** the menu remains visible

- **WHEN** user clicks elsewhere in the app or presses Escape
- **THEN** the menu closes

### Requirement: Right-click session row to open context menu with action set

Right-clicking a session row in the sidebar SHALL open a context menu offering Rename, Archive, and Delete.

#### Scenario: Right-click session opens context menu

- **WHEN** user right-clicks a session name in the sidebar
- **THEN** a context menu appears at the cursor position with the following items: "Rename", "Archive", "Delete"

#### Scenario: Rename action from session context menu

- **WHEN** user right-clicks a session and clicks "Rename"
- **THEN** the session name enters inline-edit mode

#### Scenario: Archive action from session context menu

- **WHEN** user right-clicks a session and clicks "Archive"
- **THEN** the session is soft-deleted and removed from the active sessions list

#### Scenario: Delete action from session context menu

- **WHEN** user right-clicks a session and clicks "Delete"
- **THEN** the delete confirmation dialog appears

#### Scenario: Session context menu closes on outside click

- **WHEN** user opens the session context menu
- **THEN** the menu remains visible

- **WHEN** user clicks elsewhere in the app or presses Escape
- **THEN** the menu closes

### Requirement: Context menu is keyboard-accessible

Context-menu actions SHALL be reachable and invocable by keyboard (Tab, Arrow keys, Enter).

#### Scenario: Tab through context menu items

- **WHEN** a context menu is open
- **THEN** pressing Tab focuses the next menu item and pressing Shift+Tab focuses the previous item

#### Scenario: Invoke context menu action via keyboard

- **WHEN** a context menu item is focused (via Tab)
- **THEN** pressing Enter invokes that action (e.g., opens inline rename or delete dialog)
