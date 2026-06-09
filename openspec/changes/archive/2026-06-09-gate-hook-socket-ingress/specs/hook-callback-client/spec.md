## MODIFIED Requirements

### Requirement: Runtime-free hook callback client

The hook callback client SHALL be a single standalone native executable that requires no language-specific runtime on the agent's PATH. Because length-prefix framing cannot be produced by a portable shell one-liner, the client SHALL be a small shipped binary rather than a script that relies on default tooling. It SHALL remain small and fast to start, so invoking it on every lifecycle event adds no meaningful latency to the agent.

#### Scenario: Lifecycle event fires the configured command

- **GIVEN** the agent CLI is configured to run the hook callback command on a lifecycle event
- **WHEN** the event fires
- **THEN** the configured command runs as a standalone native executable
- **AND** it does not require any language-specific runtime to be installed or resolvable on the agent's PATH

### Requirement: Forward the lifecycle payload to the loopback receiver

The hook callback client SHALL forward the agent's lifecycle payload to the loopback hook receiver as a length-prefixed frame, wrapping the payload together with the session id and session token in the frame envelope the receiver expects.

#### Scenario: Forward over the local socket

- **GIVEN** the agent invokes the client with a lifecycle payload on standard input, and the runtime directory, session id, and session token provided via the environment
- **WHEN** the client runs
- **THEN** it SHALL derive the receiver socket path from the runtime directory, wrap the payload, session id, and session token in one frame, and write that frame to the socket
- **AND** it SHALL carry the session id and session token inside the frame envelope, not as transport headers

#### Scenario: Receiver socket absent

- **GIVEN** the receiver socket is absent or unreachable
- **WHEN** the client runs
- **THEN** it SHALL exit without error and forward nothing

### Requirement: Host resolves the client at a stable location

The host SHALL resolve the hook callback command from a single stable location of the installed client binary, so hook installation registers a path that exists after build.

#### Scenario: Host prepares the hook command at startup

- **GIVEN** the host prepares the hook callback command at startup
- **WHEN** it resolves the client
- **THEN** it SHALL point the command at the installed client binary's stable path
- **AND** it SHALL surface a typed error if the binary is absent
