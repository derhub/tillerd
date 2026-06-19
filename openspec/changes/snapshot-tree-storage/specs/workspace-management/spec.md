# workspace-management — DELTA (snapshot-tree-storage)

## MODIFIED Requirements

### Requirement: Workspace creation

The orchestrator SHALL create a workspace from a name. On creation it SHALL persist a workspace
directory (slug dir + stable id) with a `workspace.json` file containing the generated id, name,
and a `sortOrder` placing it last, and return the assigned workspace id. Ordering semantics are
delegated to the `snapshot-tree-store` capability; this requirement does not restate the
mechanism. The name MAY be empty; an empty name is allowed and stored as-is.

#### Scenario: Workspace created and returned

- **WHEN** a create-workspace request supplies a name
- **THEN** the orchestrator persists a workspace directory with `workspace.json` carrying that name and returns the new workspace id

#### Scenario: New workspace ordered last

- **WHEN** a workspace is created while other workspaces exist
- **THEN** its `sortOrder` in `workspace.json` places it after all existing workspaces in the list

### Requirement: Workspace rename

The orchestrator SHALL rename a workspace by id with a new name, persisted immediately to
`workspace.json` and reflected in subsequent list and get responses. Renaming an unknown workspace
SHALL return a typed not-found error. The Default workspace MAY be renamed.

#### Scenario: Rename persists

- **WHEN** a rename-workspace request supplies a valid id and a new name
- **THEN** the name field in `workspace.json` is updated and the updated name is returned

#### Scenario: Rename unknown workspace returns error

- **WHEN** a rename-workspace request supplies an id that does not exist
- **THEN** the orchestrator returns a typed not-found error

### Requirement: Workspace listing and ordering

The orchestrator SHALL return all workspaces ordered by `sortOrder` ascending (from
`workspace.json`), falling back to creation time for entries with no explicit order. The Default
workspace SHALL always be present in the list.

#### Scenario: Ordered by sortOrder

- **WHEN** two workspaces exist with explicit `sortOrder` values
- **THEN** the workspace with the lower `sortOrder` appears first

#### Scenario: Default always present

- **WHEN** the workspace list is requested on a fresh store
- **THEN** the response contains the Default workspace

### Requirement: Workspace reorder

The orchestrator SHALL set a workspace's `sortOrder` field in `workspace.json` by id and persist
it atomically, so the new order is returned by subsequent listings and survives a restart.
Ordering uniqueness and write serialization are managed by the `snapshot-tree-store` capability.

#### Scenario: Reorder changes list order

- **WHEN** a reorder-workspace request moves a workspace to a new position
- **THEN** a subsequent workspace list reflects the new order

#### Scenario: Order persists across restart

- **WHEN** workspaces are reordered and the store is reopened
- **THEN** the persisted order is unchanged

### Requirement: Default workspace is non-deletable

The orchestrator SHALL maintain a built-in "Default" workspace with a fixed well-known id,
represented as a workspace directory with `workspace.json`. Any attempt to delete the Default
workspace SHALL be rejected with a typed error.

#### Scenario: Delete Default rejected

- **WHEN** a delete-workspace request targets the Default workspace id
- **THEN** the orchestrator returns a typed error and the Default workspace is unchanged

### Requirement: Workspace delete reassigns its projects

The orchestrator SHALL delete a non-Default workspace by id. Deleting a workspace SHALL move
every project directory it contains into the Default workspace directory (via the
`snapshot-tree-store` subtree move operation) rather than deleting those projects. Sessions and
surfaces are unaffected.

#### Scenario: Projects move to Default on delete

- **WHEN** a workspace containing projects is deleted
- **THEN** the workspace directory is removed and each of its project directories now resides under the Default workspace

#### Scenario: Delete empty workspace

- **WHEN** a workspace with no projects is deleted
- **THEN** the workspace directory is removed

### Requirement: Existing projects migrate into Default

The migration that introduces workspaces SHALL move every pre-existing project entry into the
Default workspace directory, with no data loss and no change to sessions or surfaces.

#### Scenario: Pre-existing projects land in Default

- **WHEN** a store created before the workspace migration is opened after it
- **THEN** every previously existing project belongs to the Default workspace and all sessions and surfaces are intact
