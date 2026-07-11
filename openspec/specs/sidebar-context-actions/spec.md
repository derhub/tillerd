# sidebar-context-actions Specification

## Purpose
Right-click context menus on sidebar project and session rows: project rows expose Rename, Archive, Delete, and Open in new window; session rows expose Rename, Archive, and Delete. The menu opens at the cursor, closes on outside click or Escape, and is keyboard-navigable via Tab/Shift+Tab and Enter.
## Requirements
### Requirement: Right-click project row to open context menu with full action set

Right-clicking a project row in the sidebar SHALL open a context menu offering Rename,
Duplicate, Pin (or Unpin), Move to workspace, Stop surfaces, Open in new window, Archive,
and Delete. The menu SHALL be rendered with the shared menu primitive and its items SHALL
be projections of commands tagged for the `contextmenu` surface, scoped to the row's
entity.

#### Scenario: Right-click project opens context menu

- **WHEN** user right-clicks a project name in the sidebar
- **THEN** a context menu appears at the cursor position offering "Rename", "Duplicate",
  "Pin", "Move to workspace", "Stop surfaces", "Open in new window", "Archive", "Delete"

#### Scenario: Rename action from context menu

- **WHEN** user right-clicks a project and clicks "Rename"
- **THEN** the project name enters inline-edit mode (double-click behavior)

#### Scenario: Archive action from context menu

- **WHEN** user right-clicks a project and clicks "Archive"
- **THEN** the project is soft-deleted and moves to the archived section

#### Scenario: Delete action from context menu

- **WHEN** user right-clicks a project and clicks "Delete"
- **THEN** the delete confirmation dialog appears

#### Scenario: Open in new window action from context menu

- **WHEN** user right-clicks a project and clicks "Open in new window"
- **THEN** a new window opens displaying that project with its sessions, and the parent
  sidebar shows a pending-detach indicator on the project

#### Scenario: Context menu closes on outside click

- **WHEN** user opens the context menu
- **THEN** the menu remains visible

- **WHEN** user clicks elsewhere in the app or presses Escape
- **THEN** the menu closes

### Requirement: Right-click session row to open context menu with action set

Right-clicking a session row in the sidebar SHALL open a context menu offering Rename,
Duplicate, Pin (or Unpin), Move to project, Stop surfaces, Archive, and Delete, rendered
with the shared menu primitive and driven by `contextmenu`-tagged commands scoped to the
row's session.

#### Scenario: Right-click session opens context menu

- **WHEN** user right-clicks a session name in the sidebar
- **THEN** a context menu appears at the cursor position offering "Rename", "Duplicate",
  "Pin", "Move to project", "Stop surfaces", "Archive", "Delete"

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

Context-menu actions SHALL be reachable and invocable by keyboard: arrow keys move
between items, Enter invokes the focused item, and Escape closes the menu returning focus
to the trigger row.

#### Scenario: Arrow through context menu items

- **WHEN** a context menu is open
- **THEN** pressing ArrowDown focuses the next menu item and ArrowUp focuses the previous
  item

#### Scenario: Invoke context menu action via keyboard

- **WHEN** a context menu item is focused
- **THEN** pressing Enter invokes that action (e.g., opens inline rename or the delete
  dialog)

