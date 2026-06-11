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
