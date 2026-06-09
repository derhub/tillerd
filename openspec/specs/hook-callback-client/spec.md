# hook-callback-client

## Purpose

Defines the hook callback client: the standalone executable the agent CLI runs on a lifecycle event to forward its payload to the loopback hook receiver. The client is runtime-free, fire-and-forget, and resolved by the host from a stable in-repo location.

## Requirements

### Requirement: Runtime-free hook callback client

The hook callback client SHALL be a single standalone native executable that requires no language-specific runtime on the agent's PATH. Because length-prefix framing cannot be produced by a portable shell one-liner, the client SHALL be a small shipped binary rather than a script that relies on default tooling. It SHALL remain small and fast to start, so invoking it on every lifecycle event adds no meaningful latency to the agent.

#### Scenario: Lifecycle event fires the configured command

- **GIVEN** the agent CLI is configured to run the hook callback command on a lifecycle event
- **WHEN** the event fires
- **THEN** the configured command runs as a standalone native executable
- **AND** it does not require any language-specific runtime to be installed or resolvable on the agent's PATH

### Requirement: Forward the lifecycle payload to the loopback receiver

The hook callback client SHALL forward the agent's lifecycle payload to the gate's single socket on the `Hook` route. It SHALL open the connection with the gate's route preamble — selecting the `Hook` route and carrying the session id and session token — and then write the lifecycle payload as a length-prefixed frame using the gate's shared frame codec.

#### Scenario: Forward over the local socket

- **GIVEN** the agent invokes the client with a lifecycle payload on standard input, and the runtime directory, session id, and session token provided via the environment
- **WHEN** the client runs
- **THEN** it SHALL derive the gate socket path from the runtime directory, open the connection with a route preamble selecting the `Hook` route and carrying the session id and session token, and write the payload frame to the socket
- **AND** it SHALL carry the session id and session token inside the route preamble, not as transport headers

#### Scenario: Gate socket absent

- **GIVEN** the gate socket is absent or unreachable
- **WHEN** the client runs
- **THEN** it SHALL exit without error and forward nothing

### Requirement: Fire-and-forget delivery never blocks the agent

The hook callback client SHALL never block or fail the agent, so a slow or unavailable receiver
cannot stall the session.

#### Scenario: Receiver slow, unreachable, or erroring

- **GIVEN** the receiver is slow, unreachable, or returns an error
- **WHEN** the client forwards a payload
- **THEN** it bounds its own runtime, suppresses its own errors, and always exits successfully
- **AND** the agent's hook step is never delayed beyond a short bound nor failed

### Requirement: Host resolves the client at a stable location

The host SHALL resolve the hook callback command from a single stable location of the installed client binary, so hook installation registers a path that exists after build.

#### Scenario: Host prepares the hook command at startup

- **GIVEN** the host prepares the hook callback command at startup
- **WHEN** it resolves the client
- **THEN** it SHALL point the command at the installed client binary's stable path
- **AND** it SHALL surface a typed error if the binary is absent
