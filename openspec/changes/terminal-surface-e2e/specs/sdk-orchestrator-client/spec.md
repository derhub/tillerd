## ADDED Requirements

### Requirement: Typed terminal-surface client

The SDK SHALL provide a typed client for terminal surfaces over the orchestrator API: create a
terminal surface in a session, subscribe to a surface's raw output byte stream and its
terminal-status stream, send input, and send resize. The client SHALL route by surface identifier
and SHALL NOT connect to the daemon directly.

#### Scenario: Create a terminal surface through the SDK

- **WHEN** a consumer creates a terminal surface through the SDK
- **THEN** the SDK calls the orchestrator API and returns the surface identifier

#### Scenario: Subscribe to bytes and status

- **WHEN** a consumer subscribes to a surface
- **THEN** the SDK delivers the surface's raw output bytes and terminal-status changes as they arrive over the orchestrator event stream

#### Scenario: Send input and resize

- **WHEN** a consumer sends input or a resize for a surface
- **THEN** the SDK forwards it to the orchestrator keyed by the surface identifier
- **AND** it does not open a connection to the daemon
