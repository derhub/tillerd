## ADDED Requirements

### Requirement: Create commands carry a caller-assigned identifier

Every aggregate create command (workspace, project, session, command-library entry) SHALL accept its new entity's identifier from the caller rather than minting one internally. The command SHALL persist the entity under exactly that identifier. Consistent with command-query separation, a create command SHALL return no data — the caller already holds the identifier it supplied.

#### Scenario: A create command persists under the caller's identifier

- **WHEN** a create command is dispatched with a caller-supplied identifier
- **THEN** the new entity is persisted under that exact identifier and the command returns no data

#### Scenario: Every aggregate's create follows the same identity rule

- **WHEN** a workspace, project, session, or command-library entry is created
- **THEN** each create accepts the identifier from its caller using the same contract, with no per-aggregate internal minting

### Requirement: Only identity is caller-assigned

A create command SHALL take only the entity identifier from the caller. All other derived or server-owned fields — creation timestamp, inferred name, and any launch spec resolved from a template — SHALL continue to be assigned by the command's own logic.

#### Scenario: Server-owned fields are not supplied by the caller

- **WHEN** a create command runs with a caller-supplied identifier
- **THEN** the creation timestamp, any inferred name, and any template-resolved spec are assigned by the command, not taken from the caller

### Requirement: Create handlers return the entity by reading back the assigned identifier

A control-surface handler that creates an entity and returns its record SHALL read the entity back by the caller-assigned identifier. The handler SHALL NOT determine the new entity by snapshotting the entity list before the create and diffing it against the list afterward.

#### Scenario: The created record is fetched by its known identifier

- **WHEN** a create handler completes the create and needs to return the new record
- **THEN** it fetches the entity directly by the assigned identifier and returns it

#### Scenario: Concurrent creates do not confuse each other

- **WHEN** two create handlers run concurrently against the same collection
- **THEN** each returns the record matching its own assigned identifier, with no dependence on a before/after list comparison

### Requirement: Creates are idempotent on the assigned identifier

Because the identifier is fixed before the create is issued, a create SHALL be safe to repeat: re-issuing the same create with the same identifier SHALL NOT produce a second distinct entity.

#### Scenario: Repeating a create with the same identifier does not duplicate

- **WHEN** a create is issued twice with the same caller-assigned identifier
- **THEN** at most one entity exists under that identifier
