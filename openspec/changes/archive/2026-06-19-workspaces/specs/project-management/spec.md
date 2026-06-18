## ADDED Requirements

### Requirement: Project workspace membership

A project SHALL belong to exactly one workspace, recorded as `workspace_id`. On creation a
project SHALL be assigned to the workspace supplied by the caller, defaulting to the
Default workspace when none is supplied. A project's `workspace_id` SHALL never be null at
rest after the workspace migration.

#### Scenario: New project defaults to the active workspace

- **WHEN** a project is created with no explicit workspace
- **THEN** the project belongs to the Default workspace

#### Scenario: New project assigned to a given workspace

- **WHEN** a project is created with an explicit workspace id
- **THEN** the project belongs to that workspace

### Requirement: Move project between workspaces

The orchestrator SHALL move a project to a different workspace by id, persisted
immediately. Moving a project SHALL NOT affect its sessions or surfaces. Moving to an
unknown workspace SHALL return a typed not-found error.

#### Scenario: Project moves to another workspace

- **WHEN** a move-project request supplies a valid project id and target workspace id
- **THEN** the project belongs to the target workspace and its sessions are unchanged

#### Scenario: Move to unknown workspace returns error

- **WHEN** a move-project request supplies a workspace id that does not exist
- **THEN** the orchestrator returns a typed not-found error

## MODIFIED Requirements

### Requirement: Project listing

The orchestrator SHALL return non-archived projects, optionally scoped to a single
workspace by id. When scoped, only projects belonging to that workspace are returned. Each
returned project SHALL include its `workspace_id`. Results are ordered by `sort_order`
ascending, falling back to creation time descending for rows with no explicit order.

#### Scenario: Returns only active projects

- **WHEN** the project list is requested and some projects are archived
- **THEN** only non-archived projects are returned

#### Scenario: Scoped to a workspace

- **WHEN** the project list is requested scoped to a workspace id
- **THEN** only non-archived projects belonging to that workspace are returned

#### Scenario: Ordered by sort_order then creation descending

- **WHEN** the project list is requested
- **THEN** projects with an explicit `sort_order` come first in ascending order, then the
  rest by creation time descending
