# event-dispatch Specification

## Purpose
TBD - created by archiving change standardize-event-dispatch. Update Purpose after archive.
## Requirements
### Requirement: Borrowed event delivery

A domain that streams data out of the orchestrator SHALL expose it through a sink interface whose delivery method receives the event **by borrow**, not by value. The producer SHALL NOT copy or take ownership of the payload to deliver it; whether to use the data in place, copy, or clone SHALL be the subscriber's decision.

#### Scenario: Producer delivers a payload without copying

- **WHEN** a producer delivers a byte payload to a sink
- **THEN** the payload is passed as a borrow and the producer performs no allocation or copy of the payload to make the call

#### Scenario: Subscriber retains data by its own choice

- **WHEN** a subscriber needs to keep a delivered payload beyond the call
- **THEN** the subscriber copies or clones it at its own edge, and a subscriber that only forwards or inspects does so without any copy

### Requirement: Synchronous delivery invariant

Event delivery SHALL be synchronous on the producer's thread: every subscriber SHALL run inline and the borrowed event SHALL remain valid until delivery returns. The standard SHALL NOT queue, buffer, or defer the borrowed event. A subscriber that must retain, reorder, or hand the event to another thread SHALL own a copy at its own edge and SHALL be understood to leave the zero-copy path.

#### Scenario: Delivery completes before the producer continues

- **WHEN** a producer emits an event
- **THEN** all subscribers have been invoked before the emit call returns, and no borrowed payload outlives that call

#### Scenario: A retaining subscriber owns at its edge

- **WHEN** a subscriber needs the event after delivery returns
- **THEN** it allocates its own owned copy before returning, and the standard itself stores nothing

### Requirement: Fan-out to multiple subscribers

The standard SHALL provide a reusable terminal that delivers a single borrowed event to zero or more registered subscribers in registration order. Registration SHALL be safe to perform concurrently with delivery. The same borrowed event SHALL be handed to every subscriber without per-subscriber copying.

#### Scenario: One event reaches every subscriber

- **WHEN** two subscribers are registered and a producer emits one event
- **THEN** both subscribers are invoked with the same borrowed event, in the order they registered

#### Scenario: No subscribers is a no-op

- **WHEN** a producer emits an event and no subscribers are registered
- **THEN** the call returns without error and performs no work beyond the empty fan-out

### Requirement: Middleware composes in front of delivery

A layer that intercepts events (for telemetry, filtering, or transformation) SHALL implement the same sink interface and wrap an inner sink, so layers compose into a chain whose head is the sink the producer holds. Inserting or removing a layer SHALL NOT change any producer emit call site. A layer that only observes or forwards SHALL preserve the borrow and add no copy.

#### Scenario: A layer wraps the terminal without touching the producer

- **WHEN** an observing layer is placed between a producer and the fan-out terminal
- **THEN** the producer's emit call is unchanged and events flow producer → layer → fan-out → subscribers

#### Scenario: A forwarding layer adds no copy

- **WHEN** a layer inspects an event and forwards it unchanged
- **THEN** it passes the same borrow to its inner sink and allocates nothing for the payload

### Requirement: Events leave the crate as primitives

An event type that crosses the orchestrator boundary to a host SHALL carry only primitive data (identifiers as string slices, payloads as byte or string slices) and SHALL NOT expose domain value objects or infrastructure types. The sink interface SHALL be reachable by the host without depending on the orchestrator's internal infrastructure or domain modules.

#### Scenario: Host implements a sink over primitives

- **WHEN** a host implements a sink to receive a domain's events
- **THEN** it addresses entities by primitive id and reads payloads as slices, without importing any orchestrator domain or infrastructure type

### Requirement: Key-scoped sink registration and teardown

In addition to global fan-out, the dispatch standard SHALL support registering a subscriber sink under a key and removing it again, so delivery can be scoped to the sinks registered for a given key and an individual subscriber can be torn down without affecting others. Registration and teardown SHALL be safe to perform concurrently with delivery, and SHALL preserve the existing borrowed, synchronous, zero-copy delivery for each scoped sink.

#### Scenario: A keyed sink receives only its key's events

- **WHEN** sinks are registered under two different keys and an event is dispatched for one key
- **THEN** only the sink(s) registered for that key are invoked, each with the borrowed event

#### Scenario: Removing one keyed sink leaves others intact

- **WHEN** one keyed sink is removed and an event is then dispatched for that key
- **THEN** the removed sink is not invoked and any other sinks registered for that key still receive the event

#### Scenario: Registration during delivery is safe

- **WHEN** a sink is registered or removed concurrently with an in-flight dispatch
- **THEN** the dispatch completes without error and the change takes effect for subsequent events

