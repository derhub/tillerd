# project-worktree Specification

## Purpose
TBD - created by archiving change launch-system. Update Purpose after archive.
## Requirements
### Requirement: Worktree row lifecycle

The store SHALL maintain worktree rows, each associated with a project. A worktree row SHALL
carry a unique identifier, a project reference, an absolute filesystem path, an optional
branch name, an archived-at timestamp, and a creation timestamp. The row SHALL be created when
a worktree step runs and SHALL persist until explicitly archived. Archiving is soft-delete:
the row is marked with a timestamp but not removed from the store.

#### Scenario: Worktree row created when step runs

- **WHEN** a worktree step executes and the git worktree directory is created on disk
- **THEN** a worktree row is written with the project reference, absolute path, and branch name

#### Scenario: Archived worktree is not returned by list

- **WHEN** a worktree is archived
- **THEN** it does not appear in the active worktree list for its project

#### Scenario: Worktree row survives host restart

- **WHEN** a worktree row is written and the host restarts
- **THEN** the worktree row is retrievable by its identifier

### Requirement: Worktree step — create, cd, run

A worktree step attached to a launch item SHALL execute the following operations in order:
1. Run the version-control add-worktree operation at the specified path for the specified branch, creating the directory on disk.
2. Write a worktree row to the store associated with the owning project.
3. Set the resolved working directory for the launch item's surface creation to the worktree path.

If the add-worktree operation fails (branch already checked out, path already exists, not a
git repository, etc.) the step SHALL return a typed error. The launch item's failure model
applies: the error is recorded on the surface row and execution continues with the next item.

#### Scenario: Worktree directory is created on disk

- **WHEN** the worktree step runs with a valid project, branch, and path
- **THEN** the directory at the specified path is created as a linked worktree in the source repository

#### Scenario: Step failure produces typed error

- **WHEN** the add-worktree operation fails (e.g. branch already checked out)
- **THEN** the step returns a typed error; no worktree row is written and no surface is created for that item

#### Scenario: Surface cwd is set to the worktree path

- **WHEN** the worktree step succeeds
- **THEN** the surface created for that item uses the worktree path as its working directory

### Requirement: Worktree ownership by project

Each worktree row SHALL reference exactly one project. A surface row MAY reference the worktree
that was used as its working directory. The surface's worktree reference is set at creation
time and does not change.

#### Scenario: Surface records its worktree reference

- **WHEN** a surface is created as part of a launch item that ran a worktree step
- **THEN** the surface row carries the worktree identifier

#### Scenario: Surface without worktree step has null worktree reference

- **WHEN** a surface is created without a worktree step
- **THEN** the surface row's worktree reference is null

