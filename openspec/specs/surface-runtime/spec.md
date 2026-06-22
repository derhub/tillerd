# surface-runtime Specification

## Purpose
TBD - created by archiving change terminal-surface-e2e. Update Purpose after archive.
## Requirements
### Requirement: One PTY proxy per surface

The surface-runtime SHALL own exactly one proxy per terminal surface that connects the surface to a
single pseudo-terminal in the detached daemon, keyed by the surface identifier. The proxy SHALL be
the only path between a surface and its pseudo-terminal; the renderer SHALL NOT connect to the
daemon directly.

#### Scenario: Proxy established on open

- **WHEN** a terminal surface is opened
- **THEN** the runtime establishes one proxy bound to that surface identifier and a single daemon pseudo-terminal
- **AND** no second proxy exists for the same surface identifier

#### Scenario: Output reaches the renderer through the orchestrator

- **WHEN** the renderer needs the surface's output
- **THEN** it receives it through the orchestrator
- **AND** it does not open its own connection to the daemon

### Requirement: Outbound raw-byte streaming

The proxy SHALL stream its pseudo-terminal's output as raw bytes to the host through the event-sink
abstraction, preserving every byte and escape sequence without stripping, re-encoding, or
re-decoding. Each outbound chunk SHALL carry the surface identifier so the host can route it.

#### Scenario: Output forwarded unchanged

- **WHEN** the pseudo-terminal emits output
- **THEN** the proxy forwards the exact bytes to the host event-sink tagged with the surface identifier

#### Scenario: Control sequences preserved

- **WHEN** output contains control or escape sequences
- **THEN** they are delivered unchanged, with no stripping or re-decoding

### Requirement: Input send-queue

The proxy SHALL accept input for its surface and deliver it to the pseudo-terminal in arrival order.
While the proxy is not yet attached to its pseudo-terminal — during open or reconnect — input SHALL
be queued and flushed in order once attached, never dropped or reordered. When the pseudo-terminal
cannot accept writes as fast as input arrives, the runtime SHALL apply backpressure rather than
buffer without bound.

#### Scenario: Input delivered in order when attached

- **WHEN** input arrives for an attached surface
- **THEN** it is written to the pseudo-terminal in arrival order

#### Scenario: Input queued while attaching

- **WHEN** input arrives while the proxy is still attaching
- **THEN** it is queued and flushed in order once the attachment completes

#### Scenario: Backpressure under load

- **WHEN** input arrives faster than the pseudo-terminal can accept writes
- **THEN** the runtime applies backpressure instead of buffering without bound

### Requirement: Resize propagation

The proxy SHALL propagate a surface's terminal dimensions to its pseudo-terminal so the running
program observes the correct size, and SHALL apply the surface's most recent known dimensions on
attach and reattach.

#### Scenario: Resize forwarded

- **WHEN** a surface reports new terminal dimensions
- **THEN** the proxy resizes its pseudo-terminal to match

#### Scenario: Latest dimensions applied on attach

- **WHEN** a proxy attaches or reattaches
- **THEN** it applies the surface's most recent known dimensions

### Requirement: Terminal status emission

The runtime SHALL track each surface's terminal status, derive it from the daemon's terminal-status
signal, and emit status changes to the host through the event-sink tagged with the surface
identifier, independent of the byte stream. A client subscribing to a surface SHALL receive the
surface's current status without waiting for the next change.

#### Scenario: Status change emitted

- **WHEN** the daemon reports a terminal-status change for a surface
- **THEN** the runtime emits the new status to the host tagged with the surface identifier

#### Scenario: Current status on subscribe

- **WHEN** a client subscribes to a surface
- **THEN** it receives the surface's current status without waiting for the next change

### Requirement: Reconnect by surface identifier

The runtime SHALL, after a host restart, re-establish the proxy for a persisted surface whose
pseudo-terminal is still running in the daemon by attaching to that pseudo-terminal using the
surface identifier, resuming streaming without spawning a new pseudo-terminal. If the pseudo-terminal
is no longer present, the runtime SHALL surface a typed error and SHALL NOT silently attach to a
different pseudo-terminal.

#### Scenario: Reattach to a live pseudo-terminal

- **WHEN** the host restarts and a persisted surface's pseudo-terminal is still running in the daemon
- **THEN** the runtime reattaches the proxy by surface identifier and resumes streaming
- **AND** it does not spawn a new pseudo-terminal for that surface

#### Scenario: Pseudo-terminal gone after restart

- **WHEN** a persisted surface's pseudo-terminal is no longer present in the daemon
- **THEN** the runtime surfaces a typed error
- **AND** it does not silently attach to a different pseudo-terminal

### Requirement: Resume a surface by session and placement

The runtime SHALL resolve a session's surface by the pair `(session, placement)` for any surface
kind and any count, generalizing the former terminal-only, one-per-session resume. On revisit, the
UI SHALL re-attach each of a session's panels to the surface at its placement by resolving
`(session, placement)`. The lookup SHALL return at most one live surface, or report absence; a
placement with no live surface (never spawned, or closed) is a normal absence, not an error. A
closed surface SHALL NOT be resumed.

#### Scenario: Each panel re-attaches its placement's surface on revisit

- **WHEN** a session with surfaces at two distinct placements is revisited
- **THEN** each panel re-attaches the surface resolved by its `(session, placement)` pair, and neither panel shows the other placement's surface

#### Scenario: Absence at a placement is reported, not substituted

- **WHEN** `(session, placement)` resolves to no live surface
- **THEN** the runtime returns a normal absence result and does not attach a surface from a different placement

### Requirement: Detach preserves the pseudo-terminal; removal terminates it

A proxy detach caused by host shutdown or a dropped client SHALL leave the pseudo-terminal running
in the daemon so the surface can resume; the pseudo-terminal's lifetime SHALL follow the surface,
not the client connection. Removing the surface SHALL terminate its pseudo-terminal and release the
proxy. Surface creation SHALL require a caller-supplied `session_id`; the surface-runtime SHALL NOT
mint an implicit session when creating a surface. Soft-deleting a surface via the session container
SHALL mark the surface record as archived without terminating the pseudo-terminal; hard removal via
the session container SHALL terminate the pseudo-terminal.

#### Scenario: Detach keeps the pseudo-terminal alive

- **WHEN** the host shuts down or a client disconnects
- **THEN** the proxy detaches and the pseudo-terminal keeps running in the daemon

#### Scenario: Removal terminates the pseudo-terminal

- **WHEN** the surface is removed via a hard-remove operation
- **THEN** its pseudo-terminal is terminated and the proxy is released

#### Scenario: Surface creation requires caller-supplied session id

- **WHEN** a create-surface request is received
- **THEN** the surface-runtime uses the caller-supplied `session_id` to associate the surface record and does not create a new session

#### Scenario: Soft-delete does not terminate pseudo-terminal

- **WHEN** a surface is soft-deleted through a session container archive operation
- **THEN** the surface record's `deleted_at` is set and the pseudo-terminal is not terminated

### Requirement: Surface creation dispatches by kind

The "which kinds may spawn" capability rule SHALL be enforced in the app surface-spawn use case, before any persistence or runtime effect. In 0.x the only runnable kind is `terminal`; a request for a kind with no launch adapter (e.g. `diff`) SHALL be rejected with a typed validation error and SHALL create neither a surface row nor a runtime proxy. The surface runtime SHALL be kind-agnostic: given a spawn request it spawns a pseudo-terminal and yields the per-surface proxy it owns, without inspecting the surface kind.

#### Scenario: Terminal kind spawns and yields a proxy

- **WHEN** a terminal surface is created
- **THEN** the app handler persists the pending row and the runtime spawns the command and returns the proxy it stores

#### Scenario: An unsupported kind is rejected before any effect

- **WHEN** a surface of a kind with no launch adapter (e.g. `diff`) is requested
- **THEN** the app handler returns a typed validation error and no surface row and no proxy are created

#### Scenario: The runtime does not inspect kind

- **WHEN** the runtime receives a spawn request
- **THEN** it spawns a pseudo-terminal for the surface without branching on kind, and rejects only a duplicate proxy for the same surface as raw resource integrity

### Requirement: Placement hint accepted at surface creation

Surface creation SHALL record a placement slot id on the surface row, minted by the orchestrator
and unique within the session. The placement is the durable key by which a surface is resolved
for a session: a `(session, placement)` pair SHALL identify at most one live surface. The
placement has no effect on the proxy or pseudo-terminal assignment; it is the binding key between
a surface and the panel that renders it. Creating a second live surface at a placement already in
use within a session SHALL be rejected with a typed error.

#### Scenario: Placement recorded when surface is created

- **WHEN** a surface is created for a session
- **THEN** the surface row records its minted placement and the surface is resolvable by `(session, placement)`

#### Scenario: Duplicate placement within a session is rejected

- **WHEN** a surface creation call targets a placement already held by a live surface in the same session
- **THEN** the runtime returns a typed conflict error and no second surface is created

### Requirement: Worktree reference accepted at surface creation

Surface creation SHALL accept an optional worktree identifier. When present, the worktree
identifier SHALL be stored on the surface row, associating the surface with the worktree that
provides its working directory. When absent, the surface row's worktree reference SHALL be null.

#### Scenario: Worktree reference stored when provided

- **WHEN** a surface is created with a worktree identifier
- **THEN** the surface row records the worktree identifier

#### Scenario: Absent worktree reference is stored as null

- **WHEN** a surface is created without a worktree identifier
- **THEN** the surface row's worktree reference is null

