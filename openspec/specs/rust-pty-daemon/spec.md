# rust-pty-daemon Specification

## Purpose
Defines the native Rust implementation of the PTY daemon — a wire-compatible replacement for the Node reference daemon. Implements the same Unix-socket + binary-frame protocol, session lifecycle, upgrade handoff, and hook ingress using `portable-pty` and tokio, with no Node/Bun runtime dependency.
## Requirements
### Requirement: Generic interactive command launch

The daemon SHALL spawn an interactive user command inside a pseudo-terminal from a launch
config carrying an optional command, arguments, working directory, and environment. When no
command is supplied the daemon SHALL launch the user's login shell. The spawned process is the
session target: the daemon SHALL stream the process's own output — including its shell prompt,
echoed input, and live rendering — to subscribers without suppression, and SHALL NOT hide or
replace the process behind another program.

#### Scenario: Default launch is the login shell

- **WHEN** a session is spawned with no command in the launch config
- **THEN** the daemon SHALL start the user's login shell inside a pseudo-terminal and stream its
  output, including its interactive prompt

#### Scenario: Explicit command is launched as the target

- **WHEN** a session is spawned with a command, arguments, working directory, and environment
- **THEN** the daemon SHALL start that command inside a pseudo-terminal with the given arguments,
  working directory, and environment, as the session's target process

#### Scenario: Process output is not suppressed

- **WHEN** the spawned process emits a shell prompt or echoes input
- **THEN** the daemon SHALL deliver those bytes to subscribers and SHALL NOT strip the prompt or
  echo

### Requirement: Generic command resolution with login-shell environment

The daemon SHALL resolve the launch command generically: an absolute command path SHALL be used
as given; otherwise a command name SHALL be resolved against the login-shell PATH; otherwise,
when no command is supplied, the login shell SHALL be used. The daemon SHALL install the
login-shell environment at startup so spawned commands resolve and run as in a user terminal.
The daemon SHALL NOT carry any application-specific default command, hardcoded install location,
or version gate. A named command that cannot be resolved SHALL fail with a `BinaryNotFound`
error.

#### Scenario: Absolute path is used directly

- **WHEN** the launch command is an absolute path to an executable
- **THEN** the daemon SHALL launch that executable without further resolution

#### Scenario: Command name resolved via login-shell PATH

- **WHEN** the launch command is a bare name present on the login-shell PATH
- **THEN** the daemon SHALL resolve it via the login-shell environment and launch it

#### Scenario: Unresolvable command fails typed

- **WHEN** a named launch command cannot be resolved by any method
- **THEN** the daemon SHALL emit a `BinaryNotFound` error

#### Scenario: No application-specific resolution

- **WHEN** the launch config supplies no command
- **THEN** the daemon SHALL default to the login shell and SHALL NOT substitute any
  application-specific default binary

### Requirement: Bidirectional raw byte I/O

The daemon SHALL stream PTY output bytes to subscribers and write input bytes to the PTY without
ANSI stripping, re-decoding, interrupt-key interpretation, or any transformation. Input bytes
SHALL be forwarded verbatim; the daemon SHALL NOT assign special meaning to any input byte.

#### Scenario: Output bytes pass through unmodified

- **WHEN** a session emits arbitrary bytes including escape sequences and non-UTF-8 sequences
- **THEN** the daemon SHALL deliver those exact bytes to subscribers without modification

#### Scenario: Input bytes are written verbatim

- **WHEN** raw input bytes are forwarded to a session
- **THEN** the daemon SHALL write those exact bytes to the pseudo-terminal with no special-key
  interpretation

### Requirement: Terminal resize propagation

The daemon SHALL resize the underlying pseudo-terminal on request so the spawned process
re-renders for the new dimensions.

#### Scenario: Apply new dimensions

- **WHEN** a resize to given columns and rows is requested for a session
- **THEN** the daemon SHALL set the pseudo-terminal dimensions accordingly

### Requirement: Wire-compatible reuse of the daemon protocol surface

The daemon SHALL reuse the existing daemon wire surface unchanged: the same control socket and
manifest paths honoring the base-directory override; the same length-prefixed framing (a length
prefix, then a JSON metadata header, then an optional raw binary body separated by a single
newline byte); the same `{ pid, version }` manifest shape written atomically and removed on clean
stop; and a session registry keyed by session id supporting spawn, kill, and list over the IPC
channel. Frames it produces SHALL be decodable by the reference framing decoder and vice versa.

#### Scenario: Framing is byte-compatible

- **WHEN** the daemon encodes any frame
- **THEN** the frame SHALL be decodable by the reference framing decoder, and reference-encoded
  frames SHALL be decodable by the daemon, with metadata and any raw body preserved exactly

#### Scenario: Manifest and socket paths are identical

- **WHEN** the daemon starts
- **THEN** it SHALL write the manifest to the same deterministic path with the same `{ pid,
  version }` shape, expose the control socket at the same deterministic path, and honor the same
  base-directory override

#### Scenario: Registry supports spawn, kill, and list

- **WHEN** the engine sends spawn, kill, and list commands over the IPC channel
- **THEN** the daemon SHALL start a managed session on spawn, terminate and evict it on kill, and
  return the live session ids on list

### Requirement: Replay buffer and snapshot production

The daemon SHALL maintain a bounded per-session ring buffer of raw output bytes, evicting oldest
bytes and never growing unbounded, and SHALL use it as the reconnect payload for clients that do
not negotiate the snapshot capability. For snapshot-capable clients the daemon SHALL emit a
`snapshot` frame `{ type, sessionId, rows, cols, cells, cursor }` before any new data events,
using the contracted cell encoding: the closed integer color scheme (terminal default, ANSI
standard and bright colors, 256-color palette offset, 24-bit RGB), the attribute bitmask, the
wide-character continuation and erased-cell conventions, and zero-based cursor coordinates. Given
equivalent output, the snapshot SHALL render equivalently to the reference daemon's.

#### Scenario: Capable client receives a snapshot first

- **WHEN** a snapshot-capable client subscribes to an existing session
- **THEN** the daemon SHALL emit a `snapshot` frame with the current terminal state before any
  new data events and SHALL NOT emit raw ring-buffer bytes to that client

#### Scenario: Non-capable client receives the ring buffer

- **WHEN** a client that did not negotiate the snapshot capability subscribes to an existing
  session
- **THEN** the daemon SHALL emit the ring-buffer replay before any new data events

#### Scenario: Buffer is bounded

- **WHEN** output exceeds the buffer capacity
- **THEN** the daemon SHALL evict the oldest bytes and continue, never growing unbounded

#### Scenario: Snapshot encoding matches the closed integer scheme

- **WHEN** a cell is colored with a 256-color palette index or a 24-bit RGB value
- **THEN** the daemon SHALL encode it using exactly the contracted integer scheme, identical to
  the reference daemon, with wide-character continuation cells carrying the empty string

### Requirement: Exit qualifier translation

The daemon SHALL translate every session exit into a single platform-independent exit qualifier
and include it as the primary exit field in the exit event, attaching the raw exit code and
signal only as diagnostic data. Translation precedence SHALL be: if a kill or stop command was
received, `stopped-by-request`; else for a signal-free exit, `ok` for code zero and `error`
otherwise; else the terminating signal's category SHALL select the qualifier; else `unknown`.

#### Scenario: Kill or stop yields stopped-by-request

- **WHEN** a kill or stop command is received for a session and it subsequently exits
- **THEN** the exit event SHALL carry qualifier `stopped-by-request` regardless of the underlying
  code or signal

#### Scenario: Self-exit maps by code

- **WHEN** a session exits without a preceding kill or stop command and without a signal
- **THEN** the exit event SHALL carry `ok` for code zero and `error` for a non-zero code

#### Scenario: Signal exit maps by category

- **WHEN** a session is terminated by a signal without a preceding kill or stop command
- **THEN** the daemon SHALL map the signal's category to the matching qualifier and preserve the
  raw signal as diagnostic data

### Requirement: Optional hook ingress capability

The daemon SHALL support hook ingress as an optional negotiated capability rather than a core
plane. When a connection negotiates the hook capability, the daemon SHALL run the loopback hook
receiver on the stable hook ingress socket, authenticate callbacks by per-session token, and
relay authenticated raw payloads over the IPC channel to subscribed clients for the session. A
connection that does not negotiate the hook capability SHALL be served without it and SHALL NOT
be rejected.

#### Scenario: Hook plane served only when negotiated

- **WHEN** a client negotiates the hook capability and an authenticated callback arrives
- **THEN** the daemon SHALL relay the raw payload over the IPC channel to subscribed clients for
  that session

#### Scenario: Plain terminal session needs no hook plane

- **WHEN** a client connects without negotiating the hook capability
- **THEN** the daemon SHALL serve the session normally and SHALL NOT require or reject based on
  the hook plane

### Requirement: Graceful shutdown and upgrade handoff

On a stop signal the daemon SHALL terminate all managed sessions using an escalating
graceful-stop-then-forced-kill strategy, emit exit events, and exit with no orphaned child
processes. A daemon upgrade SHALL preserve live sessions by adopting the running PTY child
processes from the outgoing daemon rather than terminating them, with clients reconnecting and
renegotiating capabilities.

#### Scenario: Cascade on stop signal

- **WHEN** the daemon receives a stop signal
- **THEN** it SHALL terminate every managed session with escalation, emit exit events, and exit
  with no PTY child remaining alive after the grace period

#### Scenario: Live sessions survive an upgrade

- **WHEN** the daemon binary is upgraded while sessions are running
- **THEN** the successor daemon SHALL adopt the running PTY processes and the sessions SHALL
  remain alive, with clients reconnecting and renegotiating capabilities

### Requirement: Selectable as the daemon binary without protocol change

The daemon SHALL be selectable through the engine's existing daemon-binary resolution, and
selecting it SHALL require no change to the wire framing, the snapshot cell encoding, or any
client logic. Reverting the selection SHALL return the system to the reference daemon with no
other change.

#### Scenario: Opt-in selection drives real sessions

- **WHEN** the daemon-binary resolution is pointed at this daemon
- **THEN** the engine SHALL spawn or adopt it and operate sessions normally over the unchanged
  protocol

#### Scenario: Selection is reversible

- **WHEN** the selection is reverted
- **THEN** the system SHALL resume using the reference daemon with no protocol, encoding, or
  client change

