# pty-daemon

## Purpose

Defines the detached daemon process that owns PTY sessions and the hook ingress socket, so both survive engine host process restarts. The daemon exposes a Unix domain socket for IPC with engine clients and manages the full lifecycle of all agent PTY processes.
## Requirements
### Requirement: Detached daemon process with manifest

The daemon SHALL run as a process independent of the engine's host process and SHALL write a manifest file to a deterministic path (`~/.athing/daemon.json`) containing its process identifier so the engine can detect whether a daemon is already running on startup.

#### Scenario: Daemon survives engine host process exit

- **WHEN** the engine's host process exits (gracefully or by crash)
- **THEN** the daemon process SHALL remain running and all PTY sessions SHALL remain alive

#### Scenario: Manifest written on start

- **WHEN** the daemon starts
- **THEN** it SHALL write `{ "pid": <pid> }` to `~/.athing/daemon.json` before accepting connections

#### Scenario: Manifest cleaned on stop

- **WHEN** the daemon stops gracefully
- **THEN** it SHALL remove `~/.athing/daemon.json`

### Requirement: IPC control channel

The daemon SHALL expose a Unix domain socket at `~/.athing/daemon.sock` for newline-delimited JSON message exchange with engine clients.

#### Scenario: Engine adopts running daemon

- **WHEN** the engine starts and the manifest PID is alive and the socket accepts connections
- **THEN** the engine SHALL connect to the existing daemon without spawning a new one

#### Scenario: Engine spawns daemon when absent

- **WHEN** the engine starts and no manifest exists, or the manifest PID is not alive
- **THEN** the engine SHALL spawn the daemon binary and wait until the socket accepts connections before proceeding

#### Scenario: Multiple engine clients

- **WHEN** more than one engine client connects to the daemon socket concurrently
- **THEN** the daemon SHALL serve all clients independently

### Requirement: Session registry

The daemon SHALL maintain a registry of all active PTY sessions keyed by session id and SHALL support spawning, killing, and listing sessions via the IPC channel.

#### Scenario: Spawn creates a managed session

- **WHEN** the engine sends a spawn command with a session id and launch config
- **THEN** the daemon SHALL start the PTY process, record the session in the registry, and acknowledge

#### Scenario: Kill terminates a session

- **WHEN** the engine sends a kill command for a known session id
- **THEN** the daemon SHALL terminate the PTY process and remove the session from the registry

#### Scenario: List returns live session ids

- **WHEN** the engine sends a list command
- **THEN** the daemon SHALL return the ids of all sessions currently in the registry

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

### Requirement: Hook ingress on stable socket

The daemon SHALL run the loopback hook receiver on a Unix domain socket at `~/.athing/hooks.sock`, so the receiver address is stable across engine host process restarts.

#### Scenario: Hook delivery after server restart

- **WHEN** the engine host process restarts while a PTY session is running
- **THEN** hook callbacks from that session SHALL continue to be received by the daemon's hook ingress without interruption

#### Scenario: Hook relay to engine

- **WHEN** an authenticated hook callback arrives at the daemon
- **THEN** the daemon SHALL relay the raw payload over the IPC channel to all subscribed engine clients for that session

### Requirement: Graceful daemon shutdown

On a stop signal the daemon SHALL terminate all managed PTY sessions using the same escalating signal strategy required by ADR-0007, emit exit events for each, and then exit cleanly.

#### Scenario: Cascade on SIGTERM

- **WHEN** the daemon receives SIGTERM
- **THEN** it SHALL initiate graceful termination of every managed session (escalating to forced kill after a grace period) and exit after all sessions have terminated or the global grace period expires

#### Scenario: No orphaned processes

- **WHEN** the daemon exits for any reason
- **THEN** no PTY child processes SHALL remain alive after the daemon's grace period

### Requirement: Exit qualifier translation at the daemon boundary

The daemon SHALL translate every session exit into a single platform-independent exit qualifier and include it as the primary exit field in the exit event emitted over the IPC channel. The daemon SHALL be the only component that reads raw platform exit codes and signals for this purpose; it SHALL attach the raw code and signal only as optional diagnostic data. Translation precedence SHALL be: if a kill or stop command was received, `stopped-by-request`; else for a signal-free exit, `ok` for code zero and `error` otherwise; else the terminating signal's category SHALL select the matching qualifier; else `unknown`.

#### Scenario: Kill or stop command yields stopped-by-request

- **WHEN** a kill or stop command is received for a session and the session subsequently exits
- **THEN** the exit event emitted by the daemon SHALL carry qualifier `stopped-by-request` regardless of the underlying exit code or signal

#### Scenario: Zero-code self-exit yields ok

- **WHEN** a session process exits with code zero and no signal without a preceding kill or stop command
- **THEN** the exit event emitted by the daemon SHALL carry qualifier `ok`

#### Scenario: Non-zero self-exit yields error

- **WHEN** a session process exits with a non-zero code and no signal without a preceding kill or stop command
- **THEN** the exit event emitted by the daemon SHALL carry qualifier `error`

#### Scenario: Signal exit maps by category

- **WHEN** a session process is terminated by a signal without a preceding kill or stop command
- **THEN** the daemon SHALL map the signal's category to the matching qualifier (for example a fault-category signal to `faulted`) and SHALL preserve the raw signal as diagnostic data

#### Scenario: External forced kill is distinct from a requested stop

- **WHEN** a session is terminated by a forced-termination signal with no preceding kill or stop command
- **THEN** the exit event SHALL carry qualifier `killed`, distinct from `stopped-by-request`

#### Scenario: Raw values are diagnostic only

- **WHEN** the daemon emits any exit event
- **THEN** the platform exit code and signal SHALL appear only as optional diagnostic fields and the qualifier SHALL be the primary exit field

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

### Requirement: Durable stopped-session set

The daemon SHALL persist stopped-session identifiers to the durable session-persistence store so that a stopped session remains ineligible for resume across engine, server, and daemon restarts. The daemon SHALL consult the durable record when evaluating a resume request. Any in-memory set SHALL be a bounded cache over the durable record.

#### Scenario: Stop recorded durably

- **WHEN** the daemon receives a stop command for a session
- **THEN** the daemon SHALL terminate the session and record its session id in the durable stopped-session store

#### Scenario: Stop survives daemon restart

- **WHEN** a session is stopped, the daemon is restarted, and a resume is later requested for that session id
- **THEN** the daemon SHALL reject the resume with a `SessionStopped` typed error, having consulted the durable store

#### Scenario: Bounded cache does not resurrect resumability

- **WHEN** the in-memory stopped-session cache evicts an entry that is still recorded in the durable store
- **THEN** a resume request for that session id SHALL still be rejected, because the durable record is authoritative

### Requirement: sessionId re-registration after eviction

The daemon registry SHALL accept re-registration of a session id it recently evicted on exit, so a crashed session can be recovered under the same id.

#### Scenario: Re-register an evicted session id

- **WHEN** a session exits and is evicted, then a spawn with that same session id arrives for recovery
- **THEN** the daemon SHALL accept the registration and manage the new process under that session id

