## MODIFIED Requirements

### Requirement: Loopback receiver

The hook receiver SHALL bind to a named Unix domain socket at a deterministic path (`$ATHING_DIR/gate-hook.sock`) and SHALL read length-prefixed frames using the same frame codec as the gate's other loopback faces (admin, subscribe, tool). It SHALL NOT use an HTTP transport or an ephemeral TCP port, and it SHALL NOT publish a separate address file, because the path is derivable from the runtime directory. The receiver SHALL be owned by the gate — a long-lived service — so it remains available across engine host process restarts.

#### Scenario: Receiver at a derivable path

- **WHEN** the receiver starts
- **THEN** it SHALL listen on `$ATHING_DIR/gate-hook.sock`, and that path SHALL be usable as long as the gate is running, with no address file to publish or read

#### Scenario: Receiver survives engine restart

- **WHEN** the engine host process exits and restarts
- **THEN** the receiver socket SHALL still be accepting callbacks from running sessions without any reconfiguration

#### Scenario: No ephemeral network port

- **WHEN** the receiver binds
- **THEN** it SHALL bind only the named Unix domain socket and SHALL NOT open any TCP port

### Requirement: Authenticated callbacks

The receiver SHALL verify the per-session token on every callback and SHALL reject callbacks with a missing or mismatched token. The token and session id SHALL be carried inside the callback frame's envelope, not as transport headers.

#### Scenario: Reject spoofed callback

- **WHEN** a callback arrives without the correct per-session token in its envelope
- **THEN** the receiver SHALL reject it and SHALL NOT change any session state

### Requirement: Transport-agnostic boundary

The receiver mechanism (binding, auth, routing) SHALL be generic engine code; the agent-specific knowledge SHALL be limited to the adapter's hook-install data and parse function. Any other producer that emits a valid `HookEvent` SHALL be able to drive the engine through the same dispatch path.

#### Scenario: Alternate producer

- **WHEN** a producer other than the framed-socket receiver dispatches a valid `HookEvent`
- **THEN** the engine SHALL process it identically, with no change to status or content logic
