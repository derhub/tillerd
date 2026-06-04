## ADDED Requirements

### Requirement: Zero-downtime daemon binary upgrade

The daemon SHALL support upgrading its own binary without interrupting live PTY sessions. The upgrade SHALL be initiated by the supervisor when it detects a version mismatch between the running daemon and the available binary. The upgrade SHALL use a handoff protocol: the running daemon (predecessor) serialises live session state, spawns the new binary (successor) with PTY master file descriptors inherited via stdio, then exits only after the successor acknowledges successful adoption.

#### Scenario: Sessions survive upgrade

- **WHEN** a daemon binary upgrade completes successfully
- **THEN** all PTY sessions that were alive before the upgrade SHALL remain alive and their output streams SHALL resume without interruption from the engine client's perspective

#### Scenario: Upgrade failure leaves predecessor alive

- **WHEN** the successor sends an `upgrade-nak` or fails to respond within the handoff timeout
- **THEN** the predecessor SHALL abort the upgrade, log the reason, and continue serving sessions normally

#### Scenario: Supervisor triggers upgrade

- **WHEN** the supervisor detects that the on-disk daemon binary version differs from the running daemon's reported version
- **THEN** the supervisor SHALL send an upgrade signal to the daemon to initiate the handoff sequence

### Requirement: Snapshot serialisation

Before spawning the successor the predecessor SHALL write a snapshot file to a temporary path. The snapshot SHALL contain for each live session: the session id, the child process pid, session metadata (cwd, cols, rows), the current replay buffer contents, and the index of the PTY master fd in the successor's stdio array. The snapshot SHALL be written atomically (write to temp path, then rename).

#### Scenario: Snapshot is complete

- **WHEN** the predecessor writes the snapshot
- **THEN** the snapshot SHALL include every session currently in the registry with no omissions

#### Scenario: Atomic snapshot write

- **WHEN** the predecessor writes the snapshot file
- **THEN** the file SHALL only become visible at its final path after the write is complete; a partial snapshot SHALL never be readable by the successor

### Requirement: PTY fd inheritance via stdio

The predecessor SHALL pass PTY master file descriptors to the successor as additional inherited stdio entries (starting at index 4, after stdin/stdout/stderr/ipc). Each session's fd index SHALL be recorded in the snapshot so the successor knows which inherited fd corresponds to which session.

#### Scenario: Fds accessible in successor

- **WHEN** the successor process starts in handoff-receiver mode
- **THEN** each PTY master fd listed in the snapshot SHALL be a valid open file descriptor readable from `process.stdio[fdIndex]`

#### Scenario: Successor wraps inherited fd

- **WHEN** the successor adopts a session from the snapshot
- **THEN** it SHALL wrap the inherited PTY master fd into a live session without re-spawning the underlying process

### Requirement: Upgrade-ack IPC handshake

The predecessor SHALL spawn the successor with an IPC channel (fd 3). After adopting all sessions and binding its Unix socket, the successor SHALL send an `upgrade-ack` message carrying its pid. The predecessor SHALL wait up to a configurable timeout (default 10 seconds) for the ack before aborting. Upon receiving the ack the predecessor SHALL update the manifest to the successor's pid and exit.

#### Scenario: Predecessor exits after ack

- **WHEN** the predecessor receives `upgrade-ack` from the successor
- **THEN** the predecessor SHALL update the manifest file, close its own socket, and exit with code 0

#### Scenario: Timeout triggers abort

- **WHEN** the predecessor does not receive `upgrade-ack` within the timeout window
- **THEN** the predecessor SHALL send `SIGKILL` to the successor, log the timeout, and continue running with no change to the manifest
