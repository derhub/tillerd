# workspace-persistence

## Purpose

The single durable product store owned by the orchestrator: one embedded relational store file in
the runtime directory, accessed only through the orchestrator API, with a schema version and lazy
migration runner, the product schema and its two-level id ownership, and a seeded default project.
## Requirements
### Requirement: Single durable product store owned by the orchestrator

The orchestrator SHALL own a single durable product store, held as one embedded relational store
file in the runtime directory, read and written only by the Rust backend. The renderer SHALL NOT
access the store directly; all reads and writes SHALL go through the orchestrator API. Service-local
runtime and discovery state SHALL stay out of the store.

#### Scenario: Store created on first boot

- **WHEN** the orchestrator boots and no product store exists yet
- **THEN** it creates the durable product store in the runtime directory

#### Scenario: Renderer cannot access the store directly

- **WHEN** the renderer needs product state
- **THEN** it obtains it through the orchestrator API
- **AND** it does not open or query the store directly

### Requirement: Schema version and lazy migration runner

The product store SHALL record its schema version in a dedicated metadata record, distinct from
any launch-spec version. On opening the store the orchestrator SHALL apply any pending schema
migrations in order to bring the store to the version the current binary expects before it serves
requests. If the store's schema version is newer than the binary supports, the orchestrator SHALL
surface a typed error and SHALL NOT serve against it.

#### Scenario: Fresh store initialized to the current version

- **WHEN** a new store is created
- **THEN** it is initialized to the schema version the current binary expects
- **AND** its metadata records that version

#### Scenario: Older store migrated forward on open

- **WHEN** the store's recorded schema version is older than the binary expects
- **THEN** the orchestrator applies the pending migrations in order before serving
- **AND** the store's recorded version becomes current

#### Scenario: Store newer than the binary is refused

- **WHEN** the store's recorded schema version is newer than the binary supports
- **THEN** the orchestrator surfaces a typed error
- **AND** it does not serve requests against the store

### Requirement: Product schema and two-level id ownership

The store SHALL hold the workspace entities — projects, worktrees, launch templates, sessions,
surfaces, commands, secret references, and settings — and a schema metadata record. The product
session identifier SHALL exist only in this store and SHALL NOT be exposed to backend services; the
surface identifier SHALL be the only identifier shared across backends.

The store trait SHALL additionally expose operations for the command library (list, get, create,
delete, seed) and worktree CRUD (create, list, archive), bringing the total set of entities
covered by the trait to projects, sessions, surfaces, layouts, commands, and worktrees.

#### Scenario: Product session id is not exposed to backends

- **WHEN** the orchestrator interacts with a backend service on behalf of a session
- **THEN** it passes the surface identifier
- **AND** it does not pass the product session identifier

#### Scenario: Surface id is the shared kernel

- **WHEN** a surface is created
- **THEN** its surface identifier is the identifier reused across the daemon and the gate

#### Scenario: Command library operations are available through the store trait

- **WHEN** the orchestrator needs to seed, list, get, create, or delete a command entry
- **THEN** it uses the store trait operations for the command table

#### Scenario: Worktree operations are available through the store trait

- **WHEN** the orchestrator needs to create, list, or archive a worktree
- **THEN** it uses the store trait operations for the worktree table

### Requirement: Seeded default project

On initialization the store SHALL seed a fixed-identifier default "Unfiled" project so that every
session always belongs to a project and a session's project reference is never null.

#### Scenario: Default project seeded on a fresh store

- **WHEN** a new store is initialized
- **THEN** the fixed-identifier "Unfiled" project exists
- **AND** a session created without an explicit project belongs to it

### Requirement: Terminal surface row persistence and resume

The store SHALL persist each surface as a durable row carrying its surface identifier, its owning
session reference, its kind, the metadata needed to reattach after a host restart, its placement
slot id, and an optional worktree reference. A session MAY own N surface rows; the pair
`(session, placement)` SHALL be unique among a session's live surfaces. The rows SHALL outlive a
host restart so the runtime can resume every one of a session's surfaces, each resolved by its
surface identifier and bound to its panel by placement. A pre-existing surface row with a null
placement SHALL be lazy-migrated to a minted placement on next open. The surface identifier
recorded here SHALL be the identifier shared with the daemon. Surface rows SHALL be created and
read only through the orchestrator; the renderer SHALL NOT access them directly.

#### Scenario: Multiple surface rows persist per session

- **WHEN** a session holds surfaces at two distinct placements
- **THEN** the store holds two surface rows for that session, each carrying its own placement and surface identifier

#### Scenario: Null placement is lazy-migrated to a minted placement

- **WHEN** a pre-existing surface row with a null placement is opened
- **THEN** the orchestrator mints a placement, writes it to the row, and the surface resolves by `(session, placement)` thereafter

#### Scenario: Duplicate placement within a session is rejected

- **WHEN** a surface row would be written for a placement already held by a live surface in the same session
- **THEN** the write is rejected with a typed conflict error

#### Scenario: All surfaces survive restart and resume by placement

- **WHEN** the host restarts
- **THEN** every persisted surface row for a session is available so the runtime resumes each surface and the UI binds it to the panel at its placement

#### Scenario: Closed surface is not resumed

- **WHEN** a surface is closed by a hard remove
- **THEN** its row is removed and the surface is not resumed on the next start

#### Scenario: Shared surface identifier

- **WHEN** the orchestrator reattaches a surface to the daemon
- **THEN** it uses the surface identifier recorded in the surface row

### Requirement: Session creation with optional template reference

`NewSession` SHALL accept an optional template identifier. When a template identifier is
supplied the store SHALL atomically copy the template's spec blob and version onto the new
session row. When no template identifier is supplied the spec fields on the session row SHALL
be null.

#### Scenario: NewSession without template produces null spec fields

- **WHEN** a session is created without a template reference
- **THEN** the session row's spec blob and spec version are both null

#### Scenario: NewSession with valid template copies spec atomically

- **WHEN** a session is created with a valid template reference
- **THEN** the session row's spec blob and version match the template's values at the time of creation, written in a single transaction

