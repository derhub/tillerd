# project-management Specification — DELTA

Change: `snapshot-tree-storage`

---

## MODIFIED Requirements

### Requirement: Project creation from source

The orchestrator SHALL create a project from one of three source kinds: `blank`, `local-dir`, or `git-repo`. A `blank` project has no associated path. The other two sources SHALL require a `root_path`. The orchestrator SHALL infer the project name from the source when no explicit name is provided: for `local-dir` use the directory basename; for `git-repo` use the repository name. The caller MAY supply a custom name to override inference. On creation the orchestrator SHALL write a `project.json` file into the snapshot tree (slug directory with a stable id, as owned by the `snapshot-tree-store` capability) and return the assigned project id.

#### Scenario: Blank project created without path

- **WHEN** a create-project request specifies source kind `blank` and no `root_path`
- **THEN** the orchestrator persists a `project.json` in the snapshot tree with source kind `blank`, `root_path` absent, an inferred or caller-supplied name, and returns the new project id

#### Scenario: Name inferred from local-dir basename

- **WHEN** a create-project request specifies source kind `local-dir` with `root_path` `/home/user/myapp` and no explicit name
- **THEN** the persisted project name is `myapp`

#### Scenario: Custom name overrides inference

- **WHEN** a create-project request supplies an explicit `name`
- **THEN** the persisted project name equals the supplied name, regardless of source

#### Scenario: git-repo name inferred from repository name

- **WHEN** a create-project request specifies source kind `git-repo` with a path whose `.git` config reports remote name `origin/my-repo`
- **THEN** the persisted project name is `my-repo`

---

### Requirement: Project soft-delete (archive) with cascade

The orchestrator SHALL archive a project by moving its subtree (including all session subtrees beneath it) to the `.archive/` directory in one atomic move, as provided by the `snapshot-tree-store` capability. After archiving, the project and its descendants SHALL not appear in active list responses.

#### Scenario: Archived project excluded from list

- **WHEN** a project is archived
- **THEN** it does not appear in the active project list

#### Scenario: Cascade to sessions and surfaces

- **WHEN** a project is archived and it owns sessions that own surfaces
- **THEN** all child session subtrees and their surface data are moved to `.archive/` in the same atomic operation

---

### Requirement: Project hard-delete

The orchestrator SHALL permanently remove an already-archived project and all of its archived descendant files (sessions, surfaces) by deleting the archived subtree from `.archive/`. Hard-delete SHALL be rejected if the project is not already archived.

#### Scenario: Hard-delete removes archived subtree

- **WHEN** a hard-delete-project request targets a project that is in `.archive/`
- **THEN** the project subtree and all child session and surface files are permanently removed from `.archive/`

#### Scenario: Hard-delete on active project rejected

- **WHEN** a hard-delete-project request targets a project that is not archived
- **THEN** the orchestrator returns a typed error and no files are removed

---

## REMOVED Requirements

### Requirement: (Scenario) Worktree directory kept

**Reason**: The `git-worktree` source kind is dropped (ADR-0033). The worktree entity no longer exists; working directory is just `cwd` on the session. The scenario guarding that the worktree directory is not removed on archive is therefore obsolete.

**Migration**: N/A — no filesystem cleanup logic was tied to this scenario.
