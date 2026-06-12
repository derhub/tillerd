# command-library Specification

## Purpose

The command library is a persistent store of named, reusable commands (executable + arguments + environment) that launch items and the renderer can reference by name. It seeds prebuilt entries on first open and exposes full CRUD to callers.
## Requirements
### Requirement: Prebuilt commands are seeded idempotently

The store SHALL seed its prebuilt command-library entries on first open and SHALL NOT create
duplicates on repeated opens or under concurrent opens.

#### Scenario: Repeated open does not duplicate

- **WHEN** the store is opened twice
- **THEN** each prebuilt command exists exactly once

#### Scenario: Concurrent open does not duplicate

- **WHEN** two opens seed the prebuilt commands concurrently
- **THEN** each prebuilt command exists exactly once

### Requirement: Command-library entries support full CRUD

The store SHALL support creating, getting, listing, and deleting command-library entries.

#### Scenario: Create then get

- **WHEN** a command is created and then fetched by its identifier
- **THEN** the stored command is returned

#### Scenario: Delete removes the entry

- **WHEN** a command is deleted
- **THEN** a subsequent get for that identifier returns nothing

### Requirement: Global named command table

The orchestrator SHALL maintain a global, durable table of named commands. Each command entry
SHALL carry a unique identifier, a display name, an origin (prebuilt or custom), an executable
path, an optional ordered argument list, and an optional environment variable map. The table
SHALL persist across host restarts.

#### Scenario: Created command survives restart

- **WHEN** a command is added to the library and the host restarts
- **THEN** the command is present in the library after restart

#### Scenario: Deleted command is absent

- **WHEN** a command is deleted from the library
- **THEN** it no longer appears in list or get responses

### Requirement: Prebuilt seed entries

On first initialization of a fresh store the orchestrator SHALL seed two prebuilt command
entries: one for the login shell (using the environment's default shell with login mode) and
one for the agent CLI preset (using the registered agent executable with its standard
arguments). Seeded entries SHALL have origin `prebuilt`. Seeding SHALL be idempotent: if the
entries are already present the operation SHALL succeed without creating duplicates.

#### Scenario: Prebuilt entries present after first boot

- **WHEN** the store is initialized for the first time
- **THEN** the login-shell entry and the agent CLI preset entry exist in the command library with origin `prebuilt`

#### Scenario: Seed is idempotent on repeated open

- **WHEN** the store is opened a second time against an already-initialized store
- **THEN** the prebuilt entries are not duplicated

### Requirement: Command library CRUD

The orchestrator SHALL expose operations to list all commands, retrieve a single command by
identifier, add a new custom command, and delete a command by identifier. Deleting a prebuilt
command SHALL be permitted. Retrieving a non-existent command SHALL return a typed not-found
result, not an error.

#### Scenario: List returns all non-deleted commands

- **WHEN** the library contains both prebuilt and custom entries
- **THEN** list returns all of them

#### Scenario: Get returns the matching entry

- **WHEN** a command identifier is looked up
- **THEN** the matching entry is returned

#### Scenario: Get on unknown id returns not-found

- **WHEN** a non-existent command identifier is looked up
- **THEN** a typed not-found result is returned, not a store error

#### Scenario: Custom command is added

- **WHEN** a create request carries name, executable path, args, and env
- **THEN** a new entry with origin `custom` is persisted and its identifier is returned

#### Scenario: Command is deleted

- **WHEN** a delete request names an existing command
- **THEN** the command is removed and subsequent list and get calls do not return it

### Requirement: Library-ref resolution at launch time

When a launch item references a named command, the executor SHALL resolve the reference against
the library at execution time. If the command is found, the executor uses its executable,
argument list, and environment map to start the surface. If the command is not found, the
executor records a typed error on the surface row for that item and continues with the
remaining items.

#### Scenario: Known library ref resolves to executable config

- **WHEN** a launch item names a command that exists in the library
- **THEN** the executor uses the stored executable, args, and env to start the surface

#### Scenario: Unknown library ref produces error surface

- **WHEN** a launch item names a command that does not exist in the library
- **THEN** the executor records a typed error on that item's surface row and proceeds to the next item

