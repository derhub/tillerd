## ADDED Requirements

### Requirement: Per-session output credit

Each engine client subscription SHALL maintain a per-session credit counter initialised to a configurable window (default 65536 bytes). The daemon SHALL deduct the byte length of each `data` frame from the credit counter before emitting the frame. When the credit reaches zero the daemon SHALL pause reading from that session's output source at the kernel level. The daemon SHALL resume reading when the credit is replenished.

#### Scenario: Credit exhausted pauses output

- **WHEN** the daemon has emitted bytes totalling the initial credit window for a session subscription and has received no `ack` frames
- **THEN** the daemon SHALL stop reading output from that session's source until credit is restored

#### Scenario: Ack replenishes credit

- **WHEN** the daemon receives an `ack` frame from a subscribed client carrying a `bytes` field
- **THEN** the daemon SHALL add `bytes` to that client's credit counter for the named session and, if the counter was previously zero, resume reading from the session source

#### Scenario: Credit is per-subscriber

- **WHEN** two engine clients are subscribed to the same session and one client's credit is exhausted
- **THEN** the daemon SHALL pause output only for that client; the other client SHALL continue receiving data at its own credit rate

### Requirement: Kernel-level pause

When pausing output for a session the daemon SHALL pause reading at the kernel level by stopping I/O reads on the PTY master fd or child IPC channel. The pause SHALL NOT rely on a software flag alone — the source process SHALL experience real write back-pressure once the kernel pipe buffer fills.

#### Scenario: Source back-pressure on pause

- **WHEN** the daemon pauses a session and the kernel pipe buffer fills
- **THEN** the source process's write syscall SHALL block, producing real upstream back-pressure with no data loss

#### Scenario: No data loss across pause/resume

- **WHEN** the daemon pauses then resumes a session
- **THEN** all bytes produced by the source during the pause SHALL be delivered in order after resumption; no bytes SHALL be dropped
