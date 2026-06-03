## MODIFIED Requirements

### Requirement: Replay buffer per session

The daemon SHALL maintain a bounded ring buffer of raw PTY output bytes per session for intra-session flow control and backpressure continuity, and SHALL use it as the reconnect payload for clients that do not negotiate the snapshot capability. Snapshot-capable clients receive a terminal state snapshot instead.

#### Scenario: Capable client receives snapshot on subscribe

- **WHEN** a snapshot-capable engine client subscribes to an existing session
- **THEN** the daemon SHALL emit a `snapshot` frame containing the current terminal state before any new data events, and SHALL NOT emit raw ring buffer bytes to that client

#### Scenario: Non-capable client receives ring buffer on subscribe

- **WHEN** an engine client that did not negotiate the snapshot capability subscribes to an existing session
- **THEN** the daemon SHALL emit the ring-buffer replay before any new data events

#### Scenario: Buffer is bounded

- **WHEN** PTY output exceeds the buffer capacity
- **THEN** the daemon SHALL evict the oldest bytes and continue, never growing the buffer unbounded

## ADDED Requirements

### Requirement: Additive capability negotiation

The daemon SHALL accept a capability advertisement from each engine client on connect and SHALL serve each connection according to the capabilities it advertised. The daemon SHALL NOT reject a client for lacking an optional capability; a missing capability SHALL degrade to the legacy behaviour for that feature.

#### Scenario: Capability recorded per connection

- **WHEN** an engine client connects and advertises a set of supported capabilities
- **THEN** the daemon SHALL record those capabilities for that connection and use them to select feature behaviour for that connection only

#### Scenario: Missing optional capability degrades, not rejects

- **WHEN** an engine client connects without advertising an optional capability
- **THEN** the daemon SHALL serve the legacy behaviour for that feature and SHALL NOT reject the connection

### Requirement: Session continuity across daemon upgrade

A daemon upgrade SHALL preserve all live PTY sessions by adopting the running PTY child processes from the outgoing daemon, rather than terminating them. Engine clients SHALL reconnect to the successor and re-run capability negotiation.

#### Scenario: Live sessions survive a daemon upgrade

- **WHEN** the daemon binary is upgraded while PTY sessions are running
- **THEN** the successor daemon SHALL adopt the running PTY processes and the sessions SHALL remain alive, with engines reconnecting and renegotiating capabilities

#### Scenario: Protocol change rides the handoff without session loss

- **WHEN** the successor daemon speaks a newer additive protocol than the outgoing daemon
- **THEN** adopted sessions SHALL keep running and clients SHALL negotiate the newer features on reconnect, with no session terminated to effect the change
