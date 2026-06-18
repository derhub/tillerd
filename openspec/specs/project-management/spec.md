# project-management Specification

## Purpose
TBD - created by archiving change projects-and-sessions-container. Update Purpose after archive.
## Requirements
### Requirement: Project creation from source

The orchestrator SHALL create a project from one of four source kinds: `blank`, `local-dir`, `git-repo`, or `git-worktree`. A `blank` project has no associated path. The other three sources SHALL require a `root_path`. The orchestrator SHALL infer the project name from the source when no explicit name is provided: for `local-dir` and `git-worktree` use the directory basename; for `git-repo` use the repository name. The caller MAY supply a custom name to override inference. On creation the orchestrator SHALL persist the project record and return the assigned project id.

#### Scenario: Blank project created without path

- **WHEN** a create-project request specifies source kind `blank` and no `root_path`
- **THEN** the orchestrator persists a project row with source kind `blank`, `root_path` NULL, an inferred or caller-supplied name, and returns the new project id

#### Scenario: Name inferred from local-dir basename

- **WHEN** a create-project request specifies source kind `local-dir` with `root_path` `/home/user/myapp` and no explicit name
- **THEN** the persisted project name is `myapp`

#### Scenario: Custom name overrides inference

- **WHEN** a create-project request supplies an explicit `name`
- **THEN** the persisted project name equals the supplied name, regardless of source

#### Scenario: git-repo name inferred from repository name

- **WHEN** a create-project request specifies source kind `git-repo` with a path whose `.git` config reports remote name `origin/my-repo`
- **THEN** the persisted project name is `my-repo`

### Requirement: Project rename

The orchestrator SHALL rename a project by accepting a project id and a new name. The new name SHALL be persisted immediately and reflected in subsequent list and get responses.

#### Scenario: Rename persists

- **WHEN** a rename-project request supplies a valid project id and a non-empty new name
- **THEN** the project record's name is updated and the updated name is returned

#### Scenario: Rename unknown project returns error

- **WHEN** a rename-project request supplies a project id that does not exist
- **THEN** the orchestrator returns a typed not-found error

### Requirement: Project listing

The orchestrator SHALL return non-archived projects, optionally scoped to a single workspace by id. When scoped, only projects belonging to that workspace are returned. Each returned project SHALL include its `workspace_id`. Results are ordered by `sort_order` ascending, falling back to creation time descending for rows with no explicit order.

#### Scenario: Returns only active projects

- **WHEN** the project list is requested and some projects are archived
- **THEN** only non-archived projects are returned

#### Scenario: Scoped to a workspace

- **WHEN** the project list is requested scoped to a workspace id
- **THEN** only non-archived projects belonging to that workspace are returned

#### Scenario: Ordered by sort_order then creation descending

- **WHEN** the project list is requested
- **THEN** projects with an explicit `sort_order` come first in ascending order, then the rest by creation time descending

### Requirement: Project workspace membership

A project SHALL belong to exactly one workspace, recorded as `workspace_id`. On creation a project SHALL be assigned to the workspace supplied by the caller, defaulting to the Default workspace when none is supplied. A project's `workspace_id` SHALL never be null at rest after the workspace migration.

#### Scenario: New project defaults to the active workspace

- **WHEN** a project is created with no explicit workspace
- **THEN** the project belongs to the Default workspace

#### Scenario: New project assigned to a given workspace

- **WHEN** a project is created with an explicit workspace id
- **THEN** the project belongs to that workspace

### Requirement: Move project between workspaces

The orchestrator SHALL move a project to a different workspace by id, persisted immediately. Moving a project SHALL NOT affect its sessions or surfaces. Moving to an unknown workspace SHALL return a typed not-found error.

#### Scenario: Project moves to another workspace

- **WHEN** a move-project request supplies a valid project id and target workspace id
- **THEN** the project belongs to the target workspace and its sessions are unchanged

#### Scenario: Move to unknown workspace returns error

- **WHEN** a move-project request supplies a workspace id that does not exist
- **THEN** the orchestrator returns a typed not-found error

### Requirement: Unfiled project is non-deletable

The orchestrator SHALL maintain a built-in "Unfiled" project with a fixed well-known id (`00000000-0000-0000-0000-000000000000`). Any attempt to archive or hard-delete the Unfiled project SHALL be rejected with a typed error.

#### Scenario: Archive Unfiled rejected

- **WHEN** an archive-project request targets the Unfiled project id
- **THEN** the orchestrator returns a typed error and the Unfiled project record is unchanged

#### Scenario: Hard-delete Unfiled rejected

- **WHEN** a hard-delete-project request targets the Unfiled project id
- **THEN** the orchestrator returns a typed error

### Requirement: Project soft-delete (archive) with cascade

The orchestrator SHALL soft-delete a project by setting its `deleted_at` timestamp. The soft-delete SHALL cascade: every session belonging to that project SHALL also be soft-deleted, and each of those sessions' surfaces SHALL also be soft-deleted. The cascade SHALL be applied atomically. After archiving, the project and its descendants SHALL not appear in active list responses.

#### Scenario: Archived project excluded from list

- **WHEN** a project is archived
- **THEN** it does not appear in the active project list

#### Scenario: Cascade to sessions and surfaces

- **WHEN** a project is archived and it owns sessions that own surfaces
- **THEN** all child session records and all grandchild surface records are soft-deleted in the same operation

#### Scenario: Worktree directory kept

- **WHEN** a project containing a worktree-source session is archived
- **THEN** the worktree directory on disk is not removed

### Requirement: Project hard-delete

The orchestrator SHALL permanently remove an already-archived project and all of its archived descendant records (sessions, surfaces). Hard-delete SHALL be rejected if the project is not already archived.

#### Scenario: Hard-delete removes rows

- **WHEN** a hard-delete-project request targets a project with `deleted_at` set
- **THEN** the project row and all child session and surface rows are permanently removed

#### Scenario: Hard-delete on active project rejected

- **WHEN** a hard-delete-project request targets a project that is not archived
- **THEN** the orchestrator returns a typed error and no rows are removed

