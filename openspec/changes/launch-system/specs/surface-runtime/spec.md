## MODIFIED Requirements

### Requirement: One PTY proxy per surface

The surface-runtime SHALL own exactly one proxy per terminal surface that connects the surface to a
single pseudo-terminal in the detached daemon, keyed by the surface identifier. The proxy SHALL be
the only path between a surface and its pseudo-terminal; the renderer SHALL NOT connect to the
daemon directly.

#### Scenario: Proxy established on open

- **WHEN** a terminal surface is opened
- **THEN** the runtime establishes one proxy bound to that surface identifier and a single daemon pseudo-terminal
- **AND** no second proxy exists for the same surface identifier

#### Scenario: Output reaches the renderer through the orchestrator

- **WHEN** the renderer needs the surface's output
- **THEN** it receives it through the orchestrator
- **AND** it does not open its own connection to the daemon

### Requirement: Placement hint accepted at surface creation

Surface creation SHALL accept an optional placement string. When present, the placement string
SHALL be stored on the surface row and later made available to the UI for routing the surface to
a named region of the panel tree. The placement hint has no effect on the proxy or pseudo-terminal
assignment; it is a metadata annotation only.

#### Scenario: Placement stored when surface is created with a hint

- **WHEN** a surface creation call includes a placement string
- **THEN** the surface row records that placement string

#### Scenario: Absent placement is stored as null

- **WHEN** a surface creation call does not include a placement string
- **THEN** the surface row's placement field is null

### Requirement: Worktree reference accepted at surface creation

Surface creation SHALL accept an optional worktree identifier. When present, the worktree
identifier SHALL be stored on the surface row, associating the surface with the worktree that
provides its working directory. When absent, the surface row's worktree reference SHALL be null.

#### Scenario: Worktree reference stored when provided

- **WHEN** a surface is created with a worktree identifier
- **THEN** the surface row records the worktree identifier

#### Scenario: Absent worktree reference is stored as null

- **WHEN** a surface is created without a worktree identifier
- **THEN** the surface row's worktree reference is null
