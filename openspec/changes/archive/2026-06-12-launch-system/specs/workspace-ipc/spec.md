## ADDED Requirements

### Requirement: Tauri command handlers for workspace CRUD

The desktop host SHALL expose Tauri command handlers that bridge the renderer's workspace
client to the Rust store. The handlers SHALL cover project and session lifecycle: create,
rename, list, and archive for both entity types, plus layout get and set for sessions. Each
handler SHALL accept a typed request payload, delegate to the store, and return a typed
response or a serializable error.

#### Scenario: Project create handler delegates to store

- **WHEN** the renderer invokes the project-create command with a valid name and source kind
- **THEN** the handler creates a project via the store and returns the created project record

#### Scenario: Session list handler returns non-archived sessions

- **WHEN** the renderer invokes the session-list command with an optional project filter
- **THEN** the handler returns the matching non-archived sessions from the store

#### Scenario: Session layout set and get round-trip

- **WHEN** the renderer sets a session's layout JSON via the layout-set command
- **THEN** a subsequent layout-get command for the same session returns the stored blob unchanged

### Requirement: Error mapping from store errors to serializable responses

Each command handler SHALL map typed store errors (not-found, conflict, constraint violation,
etc.) to serializable error responses that the renderer can inspect. The error SHALL carry
enough information for the renderer to distinguish not-found from conflict from unexpected
failure. The handler SHALL NOT panic on store errors; all error paths MUST produce a
serializable response.

#### Scenario: Not-found error is serialized

- **WHEN** a handler targets a project or session that does not exist
- **THEN** the response carries a serializable not-found error, not a panic or opaque failure

#### Scenario: Unfiled project guard is serialized

- **WHEN** the renderer attempts to rename or archive the built-in Unfiled project
- **THEN** the response carries a serializable constraint error

### Requirement: Command library IPC handlers

The desktop host SHALL expose Tauri command handlers for command library operations: list
commands, get a command by identifier, create a custom command, and delete a command. These
handlers SHALL follow the same typed-request / typed-response / serializable-error pattern as
the workspace handlers.

#### Scenario: List commands handler returns all library entries

- **WHEN** the renderer invokes the command-list handler
- **THEN** all non-deleted command entries are returned

#### Scenario: Create command handler persists a custom entry

- **WHEN** the renderer invokes the command-create handler with name, executable, args, and env
- **THEN** the entry is persisted with origin `custom` and its identifier is returned
