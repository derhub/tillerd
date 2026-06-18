## ADDED Requirements

### Requirement: Workspace creation

The orchestrator SHALL create a workspace from a name. On creation it SHALL persist the
workspace record with a generated id and a `sort_order` placing it last, and return the
assigned workspace id. The name MAY be empty; an empty name is allowed and stored as-is.

#### Scenario: Workspace created and returned

- **WHEN** a create-workspace request supplies a name
- **THEN** the orchestrator persists a workspace row with that name and returns the new
  workspace id

#### Scenario: New workspace ordered last

- **WHEN** a workspace is created while other workspaces exist
- **THEN** its `sort_order` places it after all existing workspaces in the list

### Requirement: Workspace rename

The orchestrator SHALL rename a workspace by id with a new name, persisted immediately
and reflected in subsequent list and get responses. Renaming an unknown workspace SHALL
return a typed not-found error. The Default workspace MAY be renamed.

#### Scenario: Rename persists

- **WHEN** a rename-workspace request supplies a valid id and a new name
- **THEN** the workspace record's name is updated and the updated name is returned

#### Scenario: Rename unknown workspace returns error

- **WHEN** a rename-workspace request supplies an id that does not exist
- **THEN** the orchestrator returns a typed not-found error

### Requirement: Workspace listing and ordering

The orchestrator SHALL return all workspaces ordered by `sort_order` ascending, falling
back to creation time for rows with no explicit order. The Default workspace SHALL always
be present in the list.

#### Scenario: Ordered by sort_order

- **WHEN** two workspaces exist with explicit `sort_order` values
- **THEN** the workspace with the lower `sort_order` appears first

#### Scenario: Default always present

- **WHEN** the workspace list is requested on a fresh store
- **THEN** the response contains the Default workspace

### Requirement: Workspace reorder

The orchestrator SHALL set a workspace's `sort_order` by id and persist it, so the new
order is returned by subsequent listings and survives a restart.

#### Scenario: Reorder changes list order

- **WHEN** a reorder-workspace request moves a workspace to a new position
- **THEN** a subsequent workspace list reflects the new order

#### Scenario: Order persists across restart

- **WHEN** workspaces are reordered and the store is reopened
- **THEN** the persisted order is unchanged

### Requirement: Default workspace is non-deletable

The orchestrator SHALL maintain a built-in "Default" workspace with a fixed well-known id.
Any attempt to delete the Default workspace SHALL be rejected with a typed error.

#### Scenario: Delete Default rejected

- **WHEN** a delete-workspace request targets the Default workspace id
- **THEN** the orchestrator returns a typed error and the Default workspace is unchanged

### Requirement: Workspace delete reassigns its projects

The orchestrator SHALL delete a non-Default workspace by id. Deleting a workspace SHALL
reassign every project it contains to the Default workspace rather than deleting those
projects. Sessions and surfaces are unaffected.

#### Scenario: Projects move to Default on delete

- **WHEN** a workspace containing projects is deleted
- **THEN** the workspace row is removed and each of its projects now belongs to the
  Default workspace

#### Scenario: Delete empty workspace

- **WHEN** a workspace with no projects is deleted
- **THEN** the workspace row is removed

### Requirement: Existing projects migrate into Default

The schema migration that introduces workspaces SHALL assign every pre-existing project to
the Default workspace, with no data loss and no change to sessions or surfaces.

#### Scenario: Pre-existing projects land in Default

- **WHEN** a store created before the workspace migration is opened after it
- **THEN** every previously existing project belongs to the Default workspace and all
  sessions and surfaces are intact
