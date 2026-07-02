# stream-subscription Specification

## Purpose
Stream subscription over the command bus: establishing a subscription is a dispatched command that registers a client-supplied, host-agnostic sink and returns before any data flows. After registration, frames are delivered directly to sinks scoped by subscription key, by borrow and with no per-frame dispatch, and a torn-down or closed sink stops receiving frames without blocking the source or other subscribers.
## Requirements
### Requirement: A subscription is established through the bus

Establishing a stream subscription SHALL be a command dispatched through the same bus and cross-cutting middleware as other commands. The command SHALL carry a client-provided sink, SHALL register it, and SHALL return once registration completes — without waiting for any stream data. Subscription setup is therefore observable to middleware (logging, and any future authorization), exactly once per subscription.

#### Scenario: Subscribing is observed by middleware once

- **WHEN** a client establishes a subscription
- **THEN** the subscribe command passes through the dispatch middleware once, and that middleware is not invoked again for the streamed frames

#### Scenario: Subscribe returns after registration, not after streaming

- **WHEN** a client establishes a subscription
- **THEN** the call returns once the sink is registered, before any frame is delivered

### Requirement: The client provides the sink and the core stays host-agnostic

A subscription SHALL accept a sink supplied by the client/host and SHALL address it only through a host-agnostic sink interface. The core SHALL NOT name any host-specific transport type. The client SHALL own how delivered data is handled.

#### Scenario: A host supplies its own sink

- **WHEN** a host establishes a subscription with a sink backed by its own transport
- **THEN** the core registers and drives that sink through the host-agnostic interface without referencing the host's transport type

#### Scenario: A second host substitutes a different sink unchanged

- **WHEN** a different host supplies a sink backed by a different transport
- **THEN** the subscription path is unchanged except for the sink the host provides

### Requirement: Frames stream zero-copy without per-frame dispatch

After registration, each source frame SHALL be delivered to the registered sink by borrow, with no per-frame command dispatch and no copy of the payload by the core. The single owned copy SHALL occur only at the host boundary, where the sink hands the data to its transport.

#### Scenario: A frame reaches the sink without re-entering dispatch

- **WHEN** the source produces a frame for a live subscription
- **THEN** the frame is delivered to the registered sink directly, and no subscribe/command dispatch occurs for that frame

#### Scenario: The core copies nothing; the sink owns at its edge

- **WHEN** a frame is delivered to a registered sink
- **THEN** the core passes the payload by borrow, and only the sink's hand-off to its transport produces an owned copy

### Requirement: Delivery is scoped by subscription key

A subscription SHALL be scoped by a key (for example a surface or session id). A frame SHALL be delivered only to sinks registered for that frame's key.

#### Scenario: A frame reaches only its key's sink

- **WHEN** sinks are registered under keys A and B and the source produces a frame for key A
- **THEN** only the sink registered for A is invoked, and the sink for B is not

### Requirement: A subscription can be torn down without affecting the source or other subscribers

A subscription SHALL be removable. After teardown its sink SHALL receive no further frames. A sink that has been closed or dropped by its client SHALL NOT block the source or disrupt other subscriptions.

#### Scenario: After teardown no further frames arrive

- **WHEN** a subscription is torn down and the source then produces another frame for its key
- **THEN** the torn-down sink is not invoked

#### Scenario: A closed client sink does not block the source

- **WHEN** the source produces a frame for a sink whose client has closed it
- **THEN** the source is not blocked and other subscriptions for that key still receive the frame

