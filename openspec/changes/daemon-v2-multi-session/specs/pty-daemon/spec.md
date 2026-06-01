## MODIFIED Requirements

### Requirement: IPC control channel

The daemon SHALL expose a Unix domain socket at `~/.athing/daemon.sock` for binary-framed message exchange with engine clients. All messages SHALL use the length-prefixed binary frame format defined in the `daemon-wire-protocol` spec. Clients SHALL complete the `hello` / `hello-ack` version negotiation handshake before sending any other frame type.

#### Scenario: Engine adopts running daemon

- **WHEN** the engine starts and the manifest PID is alive and the socket accepts connections
- **THEN** the engine SHALL connect to the existing daemon, complete the handshake, and proceed without spawning a new daemon

#### Scenario: Engine spawns daemon when absent

- **WHEN** the engine starts and no manifest exists, or the manifest PID is not alive
- **THEN** the engine SHALL spawn the daemon binary and wait until the socket accepts connections before proceeding

#### Scenario: Multiple engine clients

- **WHEN** more than one engine client connects to the daemon socket concurrently
- **THEN** the daemon SHALL complete an independent handshake with each client and serve them independently

## ADDED Requirements

### Requirement: Manifest includes daemon version

The manifest file SHALL include the daemon's own semver string in addition to the process identifier, so the supervisor can detect a version mismatch without connecting to the socket.

#### Scenario: Version in manifest on start

- **WHEN** the daemon starts
- **THEN** the manifest SHALL contain `{ "pid": <pid>, "version": "<semver>" }` before the socket accepts connections
