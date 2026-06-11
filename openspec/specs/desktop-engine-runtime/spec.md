# desktop-engine-runtime

## Purpose

Selecting the agent transport per host — the native transport on desktop and the network transport
on web — behind one transport abstraction with identical user-facing behavior.
## Requirements
### Requirement: Pluggable transport selection

The renderer SHALL select the native transport when running as the desktop application and the
network transport when running as the web deployment, behind one transport abstraction, with
identical user-facing behavior.

#### Scenario: Selecting the transport per host

- **WHEN** the renderer runs inside the desktop application
- **THEN** it uses the native transport
- **AND** the same renderer running as a web deployment uses the network transport, with
  identical user-facing behavior

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

