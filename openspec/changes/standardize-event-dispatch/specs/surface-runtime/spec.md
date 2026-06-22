## MODIFIED Requirements

### Requirement: Outbound raw-byte streaming

The proxy SHALL stream its pseudo-terminal's output as raw bytes to the host through the event-dispatch standard, preserving every byte and escape sequence without stripping, re-encoding, or re-decoding. Each outbound chunk SHALL be delivered as a borrowed byte slice carrying the surface identifier as a primitive, and the proxy SHALL NOT copy the payload to deliver it. Delivery SHALL be synchronous: subscribers run inline and the borrowed bytes remain valid until the emit call returns.

#### Scenario: Output forwarded unchanged

- **WHEN** the pseudo-terminal emits output
- **THEN** the proxy forwards the exact bytes to the host as a borrowed slice tagged with the surface identifier, performing no copy of the payload

#### Scenario: Control sequences preserved

- **WHEN** output contains control or escape sequences
- **THEN** they are delivered unchanged, with no stripping or re-decoding

#### Scenario: Multiple subscribers receive one chunk

- **WHEN** more than one subscriber is registered for a surface's output and a chunk arrives
- **THEN** each subscriber is invoked with the same borrowed chunk, in registration order, with no per-subscriber copy

### Requirement: Terminal status emission

The runtime SHALL track each surface's terminal status, derive it from the daemon's terminal-status signal, and emit status changes to the host through the event-dispatch standard tagged with the surface identifier as a primitive, independent of the byte stream. Status SHALL be delivered as a borrowed event over the same sink as the byte stream, distinguished by event kind. A client subscribing to a surface SHALL receive the surface's current status without waiting for the next change.

#### Scenario: Status change emitted

- **WHEN** the daemon reports a terminal-status change for a surface
- **THEN** the runtime emits the new status to the host tagged with the surface identifier, over the same sink as outbound bytes

#### Scenario: Current status on subscribe

- **WHEN** a client subscribes to a surface
- **THEN** it receives the surface's current status without waiting for the next change
