## MODIFIED Requirements

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

### Requirement: Terminal surface row persistence and resume

The store SHALL persist a terminal surface as a durable row carrying its surface identifier, its
owning session reference, its kind, the metadata needed to reattach after a host restart, an
optional placement string, and an optional worktree reference. The row SHALL outlive a host
restart so the runtime can resume the surface by its surface identifier. The surface identifier
recorded here SHALL be the identifier shared with the daemon. The surface row SHALL be created
and read only through the orchestrator; the renderer SHALL NOT access it directly.

#### Scenario: Surface row written on create with placement and worktree reference

- **WHEN** a surface is created with a placement string and a worktree reference
- **THEN** the durable surface row carries the placement string and worktree reference alongside the surface identifier, session reference, and kind

#### Scenario: Surface row written on create without placement

- **WHEN** a surface is created without a placement string
- **THEN** the surface row's placement field is null

#### Scenario: Surface row survives restart

- **WHEN** the host restarts
- **THEN** the persisted surface row is available so the runtime can resume the surface by its surface identifier

#### Scenario: Removed surface is not resumed

- **WHEN** a surface is removed
- **THEN** its row is removed and the surface is not resumed on the next start

#### Scenario: Shared surface identifier

- **WHEN** the orchestrator reattaches a surface to the daemon
- **THEN** it uses the surface identifier recorded in the surface row

## ADDED Requirements

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
