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

The daemon SHALL maintain a bounded ring buffer of raw PTY output bytes per session and SHALL deliver the buffer contents to any engine client that subscribes to a session.

#### Scenario: Replay on subscribe

- **WHEN** an engine client subscribes to an existing session
- **THEN** the daemon SHALL immediately emit the replay buffer contents before any new data events

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
