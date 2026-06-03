# virtual-terminal-state Specification

## Purpose
Defines the daemon-side virtual terminal: a headless VT parser that maintains the current screen state (cell grid, cursor, attributes) per session from raw PTY output, and serves a state snapshot to snapshot-capable clients on subscribe/reconnect so a renderer can restore the screen without replaying raw byte history. The ring buffer is retained as the legacy reconnect path for non-capable clients.

## Requirements
### Requirement: On-demand terminal state reconstruction

The daemon SHALL NOT maintain a live virtual terminal per session. The hot output path SHALL forward raw bytes unmodified and retain them in the per-session ring buffer, with no parsing. A terminal screen state SHALL be reconstructed only when a snapshot is requested, by replaying the ring buffer through a fresh parser at the session's current dimensions, producing rendered cell content, cursor position, and active text attributes. Scroll-region control (DECSTBM) is out of scope for v1 — the full screen is treated as the scrolling area.

#### Scenario: Output path does not parse

- **WHEN** the PTY produces output bytes
- **THEN** the daemon SHALL retain them in the ring buffer and forward them to subscribers unmodified, without parsing them into terminal state

#### Scenario: Snapshot reconstructed at request time

- **WHEN** a snapshot is requested for a session
- **THEN** the daemon SHALL reconstruct the current screen by replaying the ring buffer through a fresh parser, reflecting all retained output up to that point, bounded by the ring-buffer window

#### Scenario: Snapshot built at current dimensions after resize

- **WHEN** the terminal has been resized and a snapshot is subsequently requested
- **THEN** the reconstructed snapshot SHALL be built at the current dimensions; no in-place grid reflow occurs because no live grid is maintained

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

