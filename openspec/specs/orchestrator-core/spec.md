# orchestrator-core

## Purpose

The embedded, runtime-agnostic orchestrator that owns the backend for a host process: a
transport-agnostic API surface of request/response methods plus outbound event streams, and a boot
lifecycle that progresses to an observable `ready` state once its durable store and supervised
services are available.

## Requirements

### Requirement: Orchestrator is a runtime-agnostic embeddable library

The orchestrator SHALL be a runtime-agnostic library with no dependency on any specific host
runtime or user-interface toolkit, so the same orchestrator can be embedded by different hosts.
A host SHALL embed exactly one orchestrator instance in-process; the orchestrator SHALL NOT run
as a separate process.

#### Scenario: Embedded in-process by a host

- **WHEN** a host process starts and constructs the orchestrator
- **THEN** the orchestrator runs in-process within that host
- **AND** no separate orchestrator process is spawned

#### Scenario: One instance per host process

- **WHEN** a host has already constructed an orchestrator instance
- **THEN** that single instance owns the backend for the host process
- **AND** the host does not construct a second instance

### Requirement: Transport-agnostic API surface

The orchestrator SHALL expose its operations as request/response methods plus outbound event
streams, independent of any transport. Outbound events SHALL be delivered through an event-sink
abstraction the host implements and binds to its own transport, so the orchestrator does not
encode how events reach a client.

#### Scenario: Host binds the event sink and receives an event

- **WHEN** the host binds its event-sink implementation and the orchestrator emits an outbound event
- **THEN** the event is delivered to the host through the event-sink abstraction
- **AND** the orchestrator does not depend on the concrete transport used

#### Scenario: Same API over a different transport

- **WHEN** a different host binds the same API to a different transport
- **THEN** the orchestrator's request/response methods and event streams behave identically
- **AND** no orchestrator code changes are required to switch transport

### Requirement: Boot lifecycle reaches an observable ready state

On boot the orchestrator SHALL progress through a defined lifecycle to a `ready` state and SHALL
expose that state so the host and clients can observe readiness. The orchestrator SHALL NOT report
`ready` until its durable store is open and its supervised services are available. A boot failure
SHALL surface as a typed error and SHALL NOT be reported as `ready`.

#### Scenario: Boot reaches ready

- **WHEN** the orchestrator boots with its store openable and its supervised services available
- **THEN** it progresses to the `ready` state
- **AND** the host and clients can observe that it is `ready`

#### Scenario: Not ready until prerequisites are met

- **WHEN** the durable store is not yet open or a supervised service is not yet available
- **THEN** the orchestrator does not report `ready`

#### Scenario: Boot failure surfaces a typed error

- **WHEN** a prerequisite cannot be satisfied during boot
- **THEN** the orchestrator surfaces a typed error
- **AND** it does not report `ready`
