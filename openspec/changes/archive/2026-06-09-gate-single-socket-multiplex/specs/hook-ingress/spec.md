## MODIFIED Requirements

### Requirement: Install hooks once, scope per session via environment

The engine SHALL install the agent's lifecycle hook a single time (non-destructively merged into the agent's settings) and SHALL differentiate sessions by injecting the gate socket path, the session id, and a per-session token into each session's process environment at launch. It SHALL provide an explicit uninstall path. The gate socket path SHALL be a single deterministic local path that does not change across engine host process restarts.

#### Scenario: Non-destructive install

- **WHEN** the hook is installed into an agent's settings that already contain user hooks
- **THEN** the engine SHALL merge its hook command without removing or overwriting the existing hooks

#### Scenario: Per-session scoping via environment

- **WHEN** a session launches
- **THEN** the engine SHALL inject the gate socket path, session id, and per-session token into that session's environment so the static hook command reports them back

#### Scenario: Socket path survives engine restart

- **WHEN** the engine host process restarts
- **THEN** the gate socket path injected into running sessions SHALL remain valid and accept callbacks

#### Scenario: Uninstall restores settings

- **WHEN** the uninstall path is run
- **THEN** the engine's hook command SHALL be removed from the agent's settings, leaving prior user hooks intact

### Requirement: Loopback receiver

The hook receiver SHALL be the `Hook` route of the gate's single named Unix domain socket at a deterministic path (`$ATHING_DIR/gate.sock`). A hook connection SHALL open with the gate's route preamble selecting the `Hook` route, after which the receiver SHALL read length-prefixed payload frames using the gate's shared frame codec. The receiver SHALL NOT use an HTTP transport or an ephemeral TCP port, SHALL NOT bind a face-specific socket, and SHALL NOT publish a separate address file, because the path is derivable from the runtime directory. The receiver SHALL be owned by the gate — a long-lived service — so it remains available across engine host process restarts.

#### Scenario: Receiver at a derivable path

- **WHEN** the receiver starts
- **THEN** it SHALL accept hook callbacks on the `Hook` route of `$ATHING_DIR/gate.sock`, and that path SHALL be usable as long as the gate is running, with no per-face socket and no address file to publish or read

#### Scenario: Receiver survives engine restart

- **WHEN** the engine host process exits and restarts
- **THEN** the gate socket SHALL still accept `Hook`-route callbacks from running sessions without any reconfiguration

#### Scenario: No ephemeral network port

- **WHEN** the gate binds
- **THEN** it SHALL bind only the single named Unix domain socket and SHALL NOT open any TCP port

### Requirement: Authenticated callbacks

The receiver SHALL verify the per-session token on every callback and SHALL reject callbacks with a missing or mismatched token. The token and session id SHALL be carried inside the connection's route preamble, not as transport headers, and the per-session token SHALL be verified for the `Hook` route before any callback is fanned out.

#### Scenario: Reject spoofed callback

- **WHEN** a hook connection's preamble carries no correct per-session token
- **THEN** the receiver SHALL refuse it and SHALL NOT change any session state
