## ADDED Requirements

### Requirement: Terminal surface row persistence and resume

The store SHALL persist a terminal surface as a durable row carrying its surface identifier, its
owning session reference, its kind, and the metadata needed to reattach after a host restart. The
row SHALL outlive a host restart so the runtime can resume the surface by its surface identifier.
The surface identifier recorded here SHALL be the identifier shared with the daemon. The surface row
SHALL be created and read only through the orchestrator; the renderer SHALL NOT access it directly.

#### Scenario: Surface row written on create

- **WHEN** a terminal surface is created
- **THEN** a durable surface row is written with its surface identifier, owning session reference, and kind

#### Scenario: Surface row survives restart

- **WHEN** the host restarts
- **THEN** the persisted surface row is available so the runtime can resume the surface by its surface identifier

#### Scenario: Removed surface is not resumed

- **WHEN** a surface is removed
- **THEN** its row is removed and the surface is not resumed on the next start

#### Scenario: Shared surface identifier

- **WHEN** the orchestrator reattaches a surface to the daemon
- **THEN** it uses the surface identifier recorded in the surface row
