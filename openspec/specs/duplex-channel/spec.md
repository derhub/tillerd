# duplex-channel Specification

## Purpose
TBD - created by archiving change duplex-channel-verb. Update Purpose after archive.
## Requirements
### Requirement: A channel is a bidirectional session with an open/send/close lifecycle

The system SHALL provide a `channel` transport verb: a named bidirectional session a client opens with a client-provided receive sink, over which the client SHALL send messages to the backend and receive a stream from the backend, and which the client SHALL close. One client handle SHALL represent the whole session (receive, send, close); the underlying open/send split SHALL NOT leak into the caller's model.

#### Scenario: Open then send then receive then close

- **WHEN** a client opens a channel, sends a message, the backend produces stream output, and the client closes the channel
- **THEN** the session is established on open, the sent message reaches the backend, the stream output reaches the client's sink, and after close the session is torn down

### Requirement: Channel open is established through the bus and observed once

Opening a channel SHALL be a command dispatched through the bus and its cross-cutting middleware exactly once, carrying the client-provided receive sink; it SHALL register the sink and return once the session is established, before any stream data flows. The open SHALL be observable to middleware (logging, future authorization) per session, not per streamed message.

#### Scenario: Opening passes through middleware once

- **WHEN** a client opens a channel
- **THEN** the open command passes through the dispatch middleware once and middleware is not invoked again for streamed receive frames or for sends

### Requirement: Client-to-backend sends are off the telemetry path

A channel's send direction SHALL carry tagged client-to-backend messages (a data message plus control messages such as resize and close). The data message SHALL NOT pass through the dispatch telemetry path — its payload SHALL NOT be captured by any span, log, or recording layer — so raw input (for example keystrokes) is never logged. Control messages MAY be ordinary commands.

#### Scenario: A data send does not appear in telemetry

- **WHEN** a client sends a data message over a channel
- **THEN** the payload reaches the backend's runtime path and no span, log entry, or recording layer captures the payload

#### Scenario: A close control send tears the session down

- **WHEN** a client sends the close control message
- **THEN** the session is torn down and no further receive frames reach the client's sink

### Requirement: The receive direction is a zero-copy passthrough

The backend-to-client receive direction SHALL reuse the stream-subscription delivery: each source frame SHALL be delivered to the client's registered sink by borrow, with no per-frame command dispatch and no copy by the core, the single owned copy occurring only at the host boundary.

#### Scenario: Receive frames flow without per-frame dispatch

- **WHEN** the backend produces a frame for an open channel
- **THEN** the frame reaches the client's registered sink with no per-frame command dispatch

### Requirement: Surface input and output are delivered over one channel

Surface (terminal) I/O SHALL be delivered over a single `channel` session: terminal output over the receive direction and terminal input (and resize) over the send direction, replacing the separate output-subscription and input commands. Observable behavior — input reaching the terminal process, output reaching the client, and ordering — SHALL be preserved.

#### Scenario: Surface input reaches the process and output reaches the client over one channel

- **WHEN** a client opens a surface channel, sends terminal input, and the process emits output
- **THEN** the input reaches the terminal process and the output reaches the client over the same channel session

