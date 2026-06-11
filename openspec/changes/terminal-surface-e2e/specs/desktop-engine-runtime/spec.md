## ADDED Requirements

### Requirement: Desktop terminal I/O flows through the orchestrator surface-runtime

On the desktop host, a terminal surface's output, input, and resize SHALL flow through the
orchestrator's surface-runtime over the native transport. The in-renderer engine SHALL NOT carry the
terminal surface's pseudo-terminal I/O.

#### Scenario: Terminal streams through the orchestrator

- **WHEN** a terminal surface streams on the desktop host
- **THEN** its output bytes, input, and resize pass through the orchestrator surface-runtime over the native transport

#### Scenario: Engine is off the terminal path

- **WHEN** the desktop terminal is active
- **THEN** no in-renderer engine carries its pseudo-terminal I/O
