## ADDED Requirements

### Requirement: Daemon link obtained through an injected transport contract

The engine SHALL obtain its connection to the pseudo-terminal daemon through a transport
contract supplied by its caller, and SHALL route all daemon interaction — sending control
messages, subscribing to a session's frames, and listing sessions — through that contract. The
engine SHALL NOT establish the connection itself.

#### Scenario: Engine routes daemon interaction through the injected transport

- **WHEN** the engine is given a transport contract and a session is started
- **THEN** the engine sends the spawn and subsequent control messages through that transport
- **AND** it receives the session's frames through that transport's subscription

#### Scenario: A substitute transport can be supplied

- **WHEN** a test or alternate host supplies its own implementation of the transport contract
- **THEN** the engine drives the session entirely through that implementation, performing no
  direct connection of its own

### Requirement: Transcript content obtained through an injected file-read contract

The engine SHALL read agent transcript data through a file-read contract supplied by its
caller, and SHALL emit structured content derived from those reads. The engine SHALL NOT read
the filesystem directly.

#### Scenario: Content is derived from the injected file-read contract

- **WHEN** a hook indicates new transcript data and a file-read contract has been supplied
- **THEN** the engine reads the new transcript bytes through that contract
- **AND** it emits the corresponding structured content events

#### Scenario: Absent transcript is reported as a typed error

- **WHEN** the file-read contract reports the transcript as absent
- **THEN** the engine emits a typed transcript-unavailable error rather than reading the
  filesystem directly

### Requirement: Host supplies startup-resolved values

The engine SHALL receive the values produced by one-time startup resolution — a connected
daemon transport, a file-read contract, a logger, the working directory, the resolved agent
invocation, and the hook callback configuration — from its caller, and SHALL NOT perform process
spawning, executable resolution, or version probing itself.

#### Scenario: Starting a session performs no in-engine environment probing

- **WHEN** a session is started
- **THEN** the engine uses the caller-supplied resolved values, including the working directory
- **AND** it does not spawn a supervisory process, resolve an executable path, or probe a tool
  version on its own

### Requirement: Diagnostics through an injected logger

The engine SHALL emit diagnostics through a logger contract supplied by its caller, and SHALL
NOT construct a host-coupled logger itself.

#### Scenario: Engine logs through the injected logger

- **WHEN** the engine emits a diagnostic during a session
- **THEN** it routes the diagnostic through the caller-supplied logger contract

### Requirement: No reliance on ambient host primitives

The engine SHALL obtain identifiers and randomness from a runtime-neutral source and the working
directory from session options, and SHALL NOT read ambient process, environment, or current-
directory globals.

#### Scenario: Engine generates identifiers without ambient globals

- **WHEN** the engine needs a session identifier or token
- **THEN** it derives them from a runtime-neutral source available in any host
- **AND** it does not read an ambient current directory or environment global

### Requirement: Connection lifecycle owned by the caller

The caller SHALL own the lifecycle of the daemon connection and process. The engine SHALL use
the supplied connected transport, and on shutdown SHALL unsubscribe its sessions and close its
use of the transport, and SHALL NOT spawn or terminate the daemon process.

#### Scenario: Engine shutdown releases the transport without killing the daemon

- **WHEN** the engine is shut down
- **THEN** it unsubscribes its active sessions and closes its use of the transport
- **AND** it does not terminate the daemon process, leaving that to the caller

### Requirement: Behavior preserved across the injection seam

The agent-session behavior observed by a client SHALL be identical whether the engine is driven
through the production contracts or through substitute implementations, so that introducing the
injection seam changes no observable session behavior.

#### Scenario: Identical session events regardless of contract implementation

- **WHEN** the same sequence of session operations is issued against the engine backed by the
  production contracts and against the engine backed by substitute contracts
- **THEN** the engine produces the same sequence of session lifecycle events in the same order
