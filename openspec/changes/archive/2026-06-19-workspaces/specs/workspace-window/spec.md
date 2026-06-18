## ADDED Requirements

### Requirement: Workspace switcher

The system SHALL present a workspace switcher listing all workspaces in their persisted
order. Selecting a workspace SHALL make it the active workspace in the current window and
re-scope the sidebar to its projects, without opening a new window.

#### Scenario: Switching to another workspace re-scopes in place

- **WHEN** the user selects a different workspace in the switcher
- **THEN** the current window's sidebar re-scopes to the chosen workspace's projects and no
  new window is opened

### Requirement: Detach a workspace into its own window

The system SHALL allow a workspace to be opened in its own window via the same detach
affordance as projects. Opening a workspace that already has a detached window SHALL focus
the existing window rather than opening a second.

#### Scenario: Detaching a workspace opens its window

- **WHEN** the user opens a workspace in a new window
- **THEN** a window opens scoped to that workspace

#### Scenario: Re-opening a detached workspace focuses its window

- **WHEN** the user opens a workspace that already has a detached window
- **THEN** that window is focused and no new window is opened

### Requirement: Sidebar scoped to the active workspace

A window's sidebar SHALL list only the projects belonging to the active workspace, in their
persisted order, and SHALL NOT show projects of other workspaces.

#### Scenario: Sidebar shows only the workspace's projects

- **WHEN** a workspace is active and a project belongs to a different workspace
- **THEN** that project does not appear in the sidebar

#### Scenario: Project created in a workspace appears in its sidebar

- **WHEN** a project is created while a workspace is active
- **THEN** the new project appears in that workspace's sidebar
