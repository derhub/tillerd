## ADDED Requirements

### Requirement: Domain entities persist as a JSON snapshot tree

The orchestrator SHALL persist domain entities — workspaces, projects, sessions — as a
directory tree under the data root, one directory per entity, with the entity's JSON file
inside. Containment SHALL encode hierarchy: a project directory lives under its workspace's
`projects/`, a session under its project's `sessions/`. Entity JSON files SHALL carry the
stable `id` and SHALL NOT carry parent-reference fields (`workspace_id`, `project_id`).

#### Scenario: Workspace, project, and session persist as nested directories

- **WHEN** a workspace is created, then a project under it, then a session under the project
- **THEN** the tree holds `workspaces/<ws-slug>/workspace.json`,
  `workspaces/<ws-slug>/projects/<proj-slug>/project.json`, and
  `.../projects/<proj-slug>/sessions/<sess-slug>/session.json`, each file containing its stable
  `id` and no parent-id field

#### Scenario: Hierarchy is read back from containment

- **WHEN** the store lists the projects of a workspace
- **THEN** it returns exactly the projects whose directories are nested under that workspace,
  derived from the directory tree, not from a stored parent reference

### Requirement: Writes are atomic via write-temp-rename

The store SHALL write every entity file by writing to a temporary file and renaming it into
place, so a reader never observes a partially written file.

#### Scenario: A failed write leaves the prior file intact

- **WHEN** a write is interrupted before the rename completes
- **THEN** the existing entity file is unchanged and no partial file is visible at the entity path

### Requirement: Ordering is explicit via sortOrder

Each entity file SHALL carry a `sortOrder`. Listing siblings SHALL return them ordered by
`sortOrder` ascending. Reordering SHALL update the affected `sortOrder` values and persist.

#### Scenario: Siblings list in sortOrder

- **WHEN** three sessions exist under a project with sortOrder 0, 1, 2
- **THEN** listing the project's sessions returns them in sortOrder order

#### Scenario: Reorder persists across reload

- **WHEN** a session's sortOrder is changed and the store is reopened
- **THEN** the new order is reflected on listing

### Requirement: Rename re-slugs via atomic subtree move

Renaming an entity SHALL re-derive its slug from the new name and move its directory (with the
entire subtree) to the new slug path atomically. The stable `id` SHALL NOT change. Slug
collisions among siblings SHALL be disambiguated by suffixing (`foo` -> `foo-2`).

#### Scenario: Renaming a project moves its subtree and keeps the id

- **WHEN** a project named "Alpha" (with sessions) is renamed to "Beta"
- **THEN** its directory becomes `projects/beta/`, its sessions move with it, and `project.json`
  keeps the same `id`

#### Scenario: Colliding slug is disambiguated

- **WHEN** a sibling slug `foo` already exists and another entity re-slugs to `foo`
- **THEN** the new directory is `foo-2`

### Requirement: Archive moves the subtree to .archive

Archiving an entity SHALL move its directory (with its subtree) into a sibling `.archive/`
directory in a single move. Listing SHALL exclude archived entities. Hard-delete SHALL remove
the archived subtree.

#### Scenario: Archiving a session moves it out of the live tree

- **WHEN** a session is archived
- **THEN** its directory is under the project's `.archive/`, and listing the project's sessions
  no longer returns it

#### Scenario: Archiving a project archives its sessions with it

- **WHEN** a project with sessions is archived
- **THEN** the whole project subtree (including its sessions) moves under `.archive/` in one move

### Requirement: An in-memory id-to-path index resolves references

At boot the store SHALL scan the tree and build an in-memory index mapping each stable `id` to
its current directory path. Reference resolution (get-by-id) SHALL go through this index. The
index SHALL be updated on create, rename, archive, and delete. Persisting the index to
operational storage is out of scope for this capability.

#### Scenario: get-by-id resolves through the scan-built index

- **WHEN** the store is reopened and an entity is fetched by its stable `id`
- **THEN** the index built from the boot scan resolves the id to the correct path and returns the
  entity

#### Scenario: The index follows a rename

- **WHEN** an entity is renamed (its path changes)
- **THEN** fetching it by the unchanged `id` resolves to the new path

### Requirement: Layout and surface bindings persist in layout.json

Each session SHALL persist its panel tree and surface bindings in a `layout.json` file in the
session directory. A surface binding SHALL be `{ id, kind, placement, cwd }`, where `cwd` is
relative to the project root path. No worktree reference SHALL be stored on a surface.

#### Scenario: Surface binding round-trips through layout.json

- **WHEN** a session's layout with a terminal surface at a placement is saved and reloaded
- **THEN** the surface binding `{ id, kind, placement, cwd }` is read back unchanged, with `cwd`
  relative to the project root

### Requirement: Placement uniqueness is enforced in store code

The store SHALL enforce that a session holds at most one live surface per `placement`. Binding a
surface to a placement already occupied in the same session SHALL be rejected with a surface
conflict, replacing the prior SQLite partial-unique-index guarantee.

#### Scenario: Duplicate placement is rejected

- **WHEN** a surface is bound to a placement already occupied by a live surface in the same session
- **THEN** the store rejects it with a surface conflict and the existing binding is unchanged

### Requirement: Domain writes are serialized in-process

The store SHALL serialize concurrent domain writes within the process so that index updates and
write-temp-rename operations stay consistent, replacing SQLite's connection-level serialization.

#### Scenario: Concurrent writes from two windows stay consistent

- **WHEN** two windows trigger domain writes to different entities concurrently
- **THEN** both writes complete, both files are intact, and the in-memory index reflects both

### Requirement: The data root is a relocatable directory

The product store path SHALL resolve to a directory (the data root), not a single database file,
so the user may relocate or sync the domain tree.

#### Scenario: Domain tree resolves under the data-root directory

- **WHEN** the orchestrator resolves the product store path
- **THEN** it is a directory under which the `workspaces/` tree lives
