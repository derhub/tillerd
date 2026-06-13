## MODIFIED Requirements

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
