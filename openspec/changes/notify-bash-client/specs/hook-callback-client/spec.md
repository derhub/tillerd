## ADDED User Stories

### User Story: Runtime-free hook callback client

As an integrator embedding the agent, I want the hook callback client to be a single standalone shell script with no language-runtime dependency, so that lifecycle callbacks fire even in agent environments that do not have the host's application runtime available.

#### Acceptance Criteria

- **Given** the agent CLI is configured to run the hook callback command on a lifecycle event
- **When** the event fires
- **Then** the configured command SHALL be a standalone executable script that relies only on tooling present by default on the target platforms, and SHALL NOT require any language-specific runtime to be installed or resolvable on the agent's PATH

### User Story: Forward the lifecycle payload to the loopback receiver

As the engine, I want the hook callback client to forward the agent's lifecycle payload unchanged to the loopback hook receiver, so that the existing hook-ingress contract continues to work without modification.

#### Acceptance Criteria

- **Given** the agent invokes the hook callback client with a lifecycle payload on standard input and the bridge address, session id, and session token provided via the environment
- **When** the client runs
- **Then** it SHALL POST the payload verbatim to the configured bridge address, carrying the session id and session token as request headers, exactly as the loopback receiver already expects

- **Given** the bridge address is a local control-channel path
- **When** the client forwards the payload
- **Then** it SHALL deliver over that local channel without opening a network port

- **Given** the bridge address is absent from the environment
- **When** the client runs
- **Then** it SHALL exit without error and forward nothing

### User Story: Fire-and-forget delivery never blocks the agent

As the user driving the agent, I want the hook callback client to never block or fail the agent, so that a slow or unavailable receiver cannot stall my session.

#### Acceptance Criteria

- **Given** the receiver is slow, unreachable, or returns an error
- **When** the hook callback client forwards a payload
- **Then** it SHALL bound its own runtime, suppress its own errors, and always exit successfully, so the agent's hook step is never delayed beyond a short bound nor failed

### User Story: Host resolves the client at a stable location

As the host, I want to resolve the hook callback command from a single stable in-repo location, so that hook installation registers a path that exists after build.

#### Acceptance Criteria

- **Given** the host prepares the hook callback command at startup
- **When** it resolves the client
- **Then** it SHALL point the command at the standalone script's stable path and SHALL surface a typed error if the script is absent

## REMOVED User Stories

### User Story: Runtime-based hook callback script

As the host, I wanted the hook callback client to be a script executed by the application's language runtime, so that it could reuse the host's runtime to read input and forward it.

#### Removal Rationale

The runtime-based client required that runtime to be present and resolvable in the agent's environment at every hook fire, coupling a frequently-spawned foreign-process leaf to the host runtime and adding per-fire startup cost. It is replaced by the runtime-free standalone client above, which preserves the same forwarding contract while removing the runtime dependency.
