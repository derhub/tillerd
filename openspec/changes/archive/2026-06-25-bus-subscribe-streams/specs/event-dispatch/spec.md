## ADDED Requirements

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
