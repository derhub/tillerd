# bus-middleware Specification

## Purpose
TBD - created by archiving change tower-bus-middleware. Update Purpose after archive.
## Requirements
### Requirement: Command and query dispatch composes cross-cutting layers

The command/query bus SHALL wrap dispatch of every command and query in an ordered stack of cross-cutting layers, composed once at bootstrap. Adding, removing, or reordering a layer SHALL NOT change any command/query handler or any dispatch call site. Each layer SHALL be able to observe an operation before and after the handler runs and to see the handler's result.

#### Scenario: A layer observes a dispatched command without changing the handler

- **WHEN** an observing layer is installed and a command is dispatched
- **THEN** the layer is invoked for that command, the handler runs unchanged, and the command's result is unaffected

#### Scenario: Layer order follows composition order

- **WHEN** two layers are composed in a defined order and an operation is dispatched
- **THEN** each layer runs in that composition order around the handler

#### Scenario: Removing a layer leaves call sites untouched

- **WHEN** a layer is removed from the bootstrap composition
- **THEN** no command, query, handler, or dispatch call site requires any edit and dispatch still succeeds

### Requirement: Error-logging layer records one structured error per failed operation

A cross-cutting layer SHALL record exactly one structured error event for each command or query that fails, carrying the operation's stable low-cardinality error code, and SHALL record no error event for an operation that succeeds. This behavior SHALL be a composed layer rather than logic inlined into the dispatch primitive.

#### Scenario: A failing command logs exactly one error event with the stable code

- **WHEN** a command fails with a known error
- **THEN** exactly one structured error event is recorded carrying that error's stable code

#### Scenario: A successful operation logs no error event

- **WHEN** a command or query succeeds
- **THEN** no error event is recorded by the error-logging layer

### Requirement: Lifecycle signals are observable on the bus

Lifecycle signals that today bypass the bus — a surface starting, and an orchestrator status change — SHALL be delivered through the bus as observable messages, so a cross-cutting layer can observe them at a single point alongside commands and queries. A producer of such a signal SHALL route it through the bus rather than emitting it directly to the host.

#### Scenario: A surface start is observable by a bus layer

- **WHEN** a surface starts
- **THEN** the start is delivered through the bus and an installed layer observes it with the surface's session context

#### Scenario: An orchestrator status change is observable by a bus layer

- **WHEN** the orchestrator's status changes (for example, becoming ready or failing to boot)
- **THEN** the status change is delivered through the bus and an installed layer observes it

### Requirement: Notification-recording layer is the single recording point

A cross-cutting layer SHALL turn observed lifecycle signals into recorded notifications at one point, and SHALL be the only place a lifecycle signal becomes a recorded notification. A given lifecycle signal SHALL produce exactly one recorded notification; no out-of-bus recorder SHALL record the same signal in parallel.

#### Scenario: An observed surface start becomes one recorded notification

- **WHEN** a surface-start signal is observed by the recording layer
- **THEN** exactly one notification is recorded for it, with the surface's session context and the time it occurred

#### Scenario: A status change is recorded once

- **WHEN** an orchestrator status change is observed by the recording layer
- **THEN** exactly one notification is recorded for it, and no second recorder records the same change

### Requirement: Raw runtime input stays off the layered dispatch path

Cross-cutting layers SHALL NOT observe surface input, resize, or attach payloads. Raw runtime I/O SHALL remain off the bus dispatch path so that no keystroke or raw input payload is ever captured by a layer's telemetry or recording.

#### Scenario: Surface input bytes never reach a bus layer

- **WHEN** surface input, resize, or attach traffic flows to the runtime
- **THEN** no installed bus layer is invoked for it and no input payload is captured by any layer

