## ADDED Requirements

### Requirement: Terminal-surface lifecycle on the API surface

The orchestrator API SHALL expose request/response methods to create a terminal surface within a
session, to send input to a surface, and to resize a surface, plus outbound event streams that
deliver a surface's raw output bytes and its terminal-status changes over the event-sink. Creating a
terminal surface without an explicit project SHALL place its session under the seeded default
project. Every method and event SHALL be keyed by the surface identifier.

#### Scenario: Create a terminal surface

- **WHEN** a client calls the create-terminal-surface method for a session
- **THEN** the orchestrator creates the surface, starts its runtime proxy, and returns the surface identifier

#### Scenario: Input and resize routed to the surface

- **WHEN** a client sends input or a resize for a surface through the API
- **THEN** the orchestrator routes it to that surface's proxy keyed by the surface identifier

#### Scenario: Output and status delivered as events

- **WHEN** a surface produces output or a terminal-status change
- **THEN** the orchestrator delivers it as an outbound event over the event-sink tagged with the surface identifier

#### Scenario: Default project when none given

- **WHEN** a terminal surface is created without an explicit project
- **THEN** its session belongs to the seeded default project
