## MODIFIED Requirements

### Requirement: Detach preserves the pseudo-terminal; removal terminates it

A proxy detach caused by host shutdown or a dropped client SHALL leave the pseudo-terminal running
in the daemon so the surface can resume; the pseudo-terminal's lifetime SHALL follow the surface,
not the client connection. Removing the surface SHALL terminate its pseudo-terminal and release the
proxy. Surface creation SHALL require a caller-supplied `session_id`; the surface-runtime SHALL NOT
mint an implicit session when creating a surface. Removing a surface binding from a session via the
session container SHALL remove the surface's entry from `layout.json` (or move it with the session
subtree when the session is archived) without terminating the pseudo-terminal; hard removal via the
session container SHALL terminate the pseudo-terminal. The file mechanism for this is owned by
`snapshot-tree-store` — see its `Session subtree moved on archive` requirement.

#### Scenario: Detach keeps the pseudo-terminal alive

- **WHEN** the host shuts down or a client disconnects
- **THEN** the proxy detaches and the pseudo-terminal keeps running in the daemon

#### Scenario: Removal terminates the pseudo-terminal

- **WHEN** the surface is removed via a hard-remove operation
- **THEN** its pseudo-terminal is terminated and the proxy is released

#### Scenario: Surface creation requires caller-supplied session id

- **WHEN** a create-surface request is received
- **THEN** the surface-runtime uses the caller-supplied `session_id` to associate the surface record and does not create a new session

#### Scenario: Soft-delete does not terminate pseudo-terminal

- **WHEN** a surface binding is removed from `layout.json` through a session container archive operation
- **THEN** the pseudo-terminal is not terminated

### Requirement: Surface creation dispatches by kind

The surface runtime SHALL bring a surface to life only through `launch_surface`, which dispatches by
the surface's kind. In 0.x the only runnable kind is `terminal`; a `terminal` surface SHALL spawn
its command through the generic spawn and yield the per-surface proxy the runtime owns. A kind with
no launch adapter (e.g. `diff`) SHALL fail with a typed unsupported-kind error and create no proxy.
Surface creation SHALL accept `{id?, session, kind, placement?, cwd?}`; `worktree_id` is not
accepted — a working directory is supplied directly as `cwd` (relative to the project root).

#### Scenario: Terminal kind spawns and yields a proxy

- **WHEN** a terminal surface is created
- **THEN** the generic spawn runs the command and returns the proxy the runtime stores

#### Scenario: An unsupported kind fails loudly

- **WHEN** a surface of a kind with no launch adapter (e.g. `diff`) is created
- **THEN** the runtime returns a typed unsupported-kind error and stores no proxy

### Requirement: Placement hint accepted at surface creation

Surface creation SHALL record a placement slot id on the surface entry in `layout.json`, minted by
the orchestrator and unique within the session. The placement is the durable key by which a surface
is resolved for a session: a `(session, placement)` pair SHALL identify at most one live surface.
The placement has no effect on the proxy or pseudo-terminal assignment; it is the binding key
between a surface and the panel that renders it. Creating a second live surface at a placement
already in use within a session SHALL be rejected with a typed error. Placement-uniqueness within a
session is enforced by `snapshot-tree-store` — see its `Placement-uniqueness enforced by the store`
requirement — rather than by a SQL unique index.

#### Scenario: Placement recorded when surface is created

- **WHEN** a surface is created for a session
- **THEN** the surface entry in `layout.json` records its minted placement and the surface is resolvable by `(session, placement)`

#### Scenario: Duplicate placement within a session is rejected

- **WHEN** a surface creation call targets a placement already held by a live surface in the same session
- **THEN** the runtime returns a typed conflict error and no second surface is created

## REMOVED Requirements

### Requirement: Worktree reference accepted at surface creation

**Reason**: The worktree entity has been dropped (ADR-0033). A surface is now `{id, kind, placement, cwd}`; there is no worktree row to reference.

**Migration**: A working directory is supplied directly as the optional `cwd` field (relative to the project root) on the surface creation request. No worktree identifier is stored or accepted.
