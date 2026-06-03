# virtual-terminal-state Specification

## Purpose
TBD - created by archiving change terminal-state-snapshot. Update Purpose after archive.
## Requirements
### Requirement: VT state maintenance

The daemon SHALL parse raw PTY output for each active session and maintain an in-memory representation of the current terminal screen state, including rendered cell content, cursor position, scroll region, and active text attributes.

#### Scenario: State updates on output

- **WHEN** the PTY produces output bytes
- **THEN** the daemon SHALL parse and apply those bytes to the session's in-memory terminal state before forwarding them to subscribers

#### Scenario: State reflects latest output

- **WHEN** any subscriber queries terminal state for an active session
- **THEN** the state SHALL reflect all output received up to that point in time

#### Scenario: Resize without reflow

- **WHEN** the terminal is resized
- **THEN** the daemon SHALL preserve cells in the overlapping region, clear newly exposed cells, and drop content beyond the new bounds, without reflowing wrapped lines

### Requirement: State snapshot on subscribe for capable clients

The daemon SHALL deliver a terminal state snapshot to any `snapshot`-capable client subscribing to or reconnecting to a session. The snapshot SHALL represent the complete current screen without requiring the client to parse raw byte history. Clients that do not negotiate the `snapshot` capability SHALL receive the legacy raw ring-buffer replay instead, so no client is rejected for lacking the feature.

#### Scenario: Capable subscriber receives snapshot

- **WHEN** a client that advertised the `snapshot` capability subscribes to an active session
- **THEN** the daemon SHALL emit a single `snapshot` frame containing the full current terminal state before emitting any further live data frames

#### Scenario: Non-capable subscriber receives legacy replay

- **WHEN** a client that did not advertise the `snapshot` capability subscribes to an active session
- **THEN** the daemon SHALL emit the legacy raw ring-buffer replay and SHALL NOT send a `snapshot` frame

#### Scenario: Snapshot completeness

- **WHEN** the snapshot is delivered
- **THEN** it SHALL encode all cell content, cursor position, and attributes required for a terminal renderer to reproduce the current screen without any prior data frames

### Requirement: Atomic snapshot-to-live seam

When a capable client subscribes, snapshot generation and live-stream attachment SHALL be atomic with respect to the session's output path, so the snapshot is an exact prefix of the session output and the live stream is the exact suffix, with no bytes lost between them and no bytes duplicated across them.

#### Scenario: No gap or overlap at the seam

- **WHEN** PTY output is arriving while a capable client subscribes
- **THEN** every output byte SHALL appear either inside the snapshot or in the subsequent live stream, exactly once, with none lost and none duplicated

### Requirement: Ring buffer retained as legacy reconnect path

The daemon SHALL retain the raw ring buffer both for intra-session backpressure continuity and as the reconnect payload for clients that do not negotiate the `snapshot` capability.

#### Scenario: Ring buffer serves non-capable clients

- **WHEN** a non-capable client subscribes
- **THEN** the daemon SHALL replay the ring-buffer contents as the reconnect payload

