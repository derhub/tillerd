# rust-platform-host Specification

## Purpose
Defines the platform host that backs the engine's platform ports with the native Rust daemon — manifest-based supervision, daemon-artifact resolution, transport over the shared codec, and file-read and agent resolution — as a default, drop-in replacement for the reference host with no engine or wire-protocol change.
## Requirements
### Requirement: Host implements the engine platform ports backed by the native daemon

The host SHALL produce the startup-resolved values the engine requires — a connected daemon
transport, a file-read source, a logger, the working directory, the resolved agent invocation,
the agent-home location, and the hook callback configuration — with the native terminal daemon
as its backend. The values it supplies SHALL satisfy the same platform-port contracts the engine
consumes, so the engine drives a session through this host with no engine change.

#### Scenario: Engine driven through the native-backed host

- **WHEN** a composition root supplies the engine with the values produced by this host
- **THEN** the engine starts, drives, and shuts down a session using only those values
- **AND** it performs no daemon connection, process spawning, or executable resolution of its own

#### Scenario: Observable session behavior matches the reference host

- **WHEN** the same sequence of session operations is issued against the engine backed by this
  host and against the engine backed by the reference host
- **THEN** the engine produces the same sequence of session lifecycle events in the same order

### Requirement: Native daemon artifact resolved by build or discovery

The host SHALL locate the native daemon's compiled artifact by an explicit override, then by
building or discovering it at its known build-output location, then by the established install
locations — and SHALL NOT depend solely on an ambient generic-binary name lookup. When the
artifact cannot be located, the host SHALL fail with a typed not-found error naming the override
and the build step.

#### Scenario: Artifact located at its build-output path

- **WHEN** the native daemon has been built and no override is set
- **THEN** the host resolves the compiled artifact from its known build-output location

#### Scenario: Explicit override wins

- **WHEN** an explicit artifact-path override is set
- **THEN** the host uses the overridden path in preference to any discovered location

#### Scenario: Missing artifact reported as a typed error

- **WHEN** no artifact can be located by override, build-output, or install location
- **THEN** the host raises a typed not-found error that names the override variable and the build
  step rather than spawning an arbitrary binary

### Requirement: Manifest-based supervision over the shared wire contract

The host SHALL supervise the native daemon by adopting a live instance recorded in the shared
manifest when one is reachable, and otherwise spawning a new detached instance and waiting for it
to become reachable within a bounded deadline. Adoption and spawning SHALL use the shared socket
paths, manifest format, and framing unchanged. When the daemon does not become reachable within
the deadline, the host SHALL fail with a typed error.

#### Scenario: Adopt a live daemon

- **WHEN** the manifest records a process that is alive and its control socket answers
- **THEN** the host connects to the existing daemon rather than spawning a new one

#### Scenario: Spawn when none is reachable

- **WHEN** no manifest exists, or the recorded process is dead or unresponsive
- **THEN** the host clears any stale socket, spawns a new detached native daemon, and connects
  once its control socket becomes reachable

#### Scenario: Startup deadline exceeded

- **WHEN** a spawned daemon does not expose a reachable control socket within the bounded deadline
- **THEN** the host raises a typed startup-timeout error

### Requirement: Daemon transport over the shared codec and framing

The host SHALL connect to the daemon's control socket, complete the version/capability handshake,
and expose a transport that sends control messages, subscribes to a session's frames, lists
sessions, and reports connection close — using the shared frame codec and length-prefixed framing
unchanged, forwarding raw body bytes without re-decoding.

#### Scenario: Handshake precedes session traffic

- **WHEN** the transport connects to the daemon
- **THEN** it completes the version/capability handshake before dispatching any session frame
- **AND** a handshake rejection surfaces as a typed transport error

#### Scenario: Session frames routed to subscribers

- **WHEN** the daemon emits frames for a subscribed session
- **THEN** the transport delivers each frame and its raw body bytes to that session's subscribers
  without altering the body

#### Scenario: Pending list calls drain on close

- **WHEN** the connection closes while a list request is outstanding
- **THEN** the transport resolves the outstanding request and notifies close handlers rather than
  hanging

### Requirement: Narrower native control plane handled by graceful degradation

The host SHALL NOT assume the native daemon implements control-plane features it omits — optional
hook ingress, a CLI version gate, turn-cancel/interrupt semantics, and live upgrade handoff. The
host SHALL rely on the shared wire contract degrading these gracefully — an unimplemented control
message is a no-op and an unstarted hook-ingress listener yields no hook frames — rather than
introducing backend-specific gating, and SHALL add no new protocol surface for them.

#### Scenario: Unimplemented control message is a no-op

- **WHEN** a caller issues a control-plane action the native daemon does not implement
- **THEN** the action is a no-op at the daemon and the session continues
- **AND** the host neither fails the session nor adds backend-specific handling for it

#### Scenario: Absent hook ingress yields no hook frames

- **WHEN** the native daemon does not run a hook-ingress listener
- **THEN** the host's hook-socket configuration simply yields no hook frames
- **AND** the session proceeds on its terminal-derived signals without error

### Requirement: File-read source and agent resolution supplied by the host

The host SHALL supply a file-read source that reads transcript bytes and reports an absent
transcript distinctly, and SHALL resolve the agent invocation from the adapter's declarative
resolution policy — an explicit override, then the login-shell path, then the policy's common
locations — raising a typed not-found error when resolution fails. These resolutions SHALL occur
in the host, not the engine.

#### Scenario: Transcript bytes read through the file-read source

- **WHEN** the engine reads transcript data through the supplied file-read source
- **THEN** the source returns the requested bytes
- **AND** when the transcript is absent the source reports absence distinctly so the engine can
  emit a typed transcript-unavailable error

#### Scenario: Agent invocation resolved from declarative policy

- **WHEN** the host resolves the agent invocation from the adapter's resolution policy
- **THEN** it honors the override, then the login-shell path, then the common locations in order
- **AND** it raises a typed not-found error naming the override when none resolves

### Requirement: Native daemon is the default backend, selectable without protocol change

A composition root SHALL select the native backend by importing this host's platform-port surface
in place of the reference host, with no change to the engine, the wire protocol, or the native
daemon. The native daemon SHALL be this host's default backend — resolved with no environment
override required — while the explicit override remains honored. The host SHALL add no new protocol
surface beyond the shared contracts.

#### Scenario: Native backend resolved by default

- **WHEN** a composition root uses this host with no daemon-binary override set
- **THEN** supervision resolves and spawns the native daemon artifact without further configuration

#### Scenario: Swap hosts without engine or protocol change

- **WHEN** a composition root replaces the reference host with this host
- **THEN** the engine and wire protocol are unchanged and the session operates over the shared
  contracts
