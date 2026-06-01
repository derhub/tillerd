## MODIFIED Requirements

### Requirement: Install hooks once, scope per session via environment

The engine SHALL install the agent's lifecycle hook a single time (non-destructively merged into the agent's settings) and SHALL differentiate sessions by injecting the hook socket path, the session id, and a per-session token into each session's process environment at launch. It SHALL provide an explicit uninstall path. The hook socket path SHALL be a deterministic local path that does not change across engine host process restarts.

#### Scenario: Non-destructive install

- **WHEN** the hook is installed into an agent's settings that already contain user hooks
- **THEN** the engine SHALL merge its hook command without removing or overwriting the existing hooks

#### Scenario: Per-session scoping via environment

- **WHEN** a session launches
- **THEN** the engine SHALL inject the hook socket path, session id, and per-session token into that session's environment so the static hook command reports them back

#### Scenario: Socket path survives engine restart

- **WHEN** the engine host process restarts
- **THEN** the hook socket path injected into running sessions SHALL remain valid and accept callbacks

#### Scenario: Uninstall restores settings

- **WHEN** the uninstall path is run
- **THEN** the engine's hook command SHALL be removed from the agent's settings, leaving prior user hooks intact

### Requirement: Loopback receiver

The hook receiver SHALL bind to a named Unix domain socket at a deterministic path (`~/.athing/hooks.sock`) on the loopback-equivalent local interface. It SHALL NOT use an ephemeral TCP port. The receiver SHALL be owned and managed by the daemon process so it remains available across engine host process restarts.

#### Scenario: Receiver at stable path

- **WHEN** the receiver starts
- **THEN** it SHALL listen on `~/.athing/hooks.sock` and that path SHALL be usable as long as the daemon is running

#### Scenario: Receiver survives engine restart

- **WHEN** the engine host process exits and restarts
- **THEN** the receiver socket SHALL still be accepting callbacks from running sessions without any reconfiguration
