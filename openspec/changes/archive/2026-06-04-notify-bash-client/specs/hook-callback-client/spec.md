## ADDED Requirements

### Requirement: Runtime-free hook callback client

The hook callback client SHALL be a single standalone executable script that relies only on
tooling present by default on the target platforms (macOS/Linux for v1), and SHALL NOT require
any language-specific runtime to be installed or resolvable on the agent's PATH.

#### Scenario: Lifecycle event fires the configured command

- **GIVEN** the agent CLI is configured to run the hook callback command on a lifecycle event
- **WHEN** the event fires
- **THEN** the configured command runs as a standalone executable script
- **AND** it does not require any language-specific runtime to be installed or resolvable on the
  agent's PATH

### Requirement: Forward the lifecycle payload to the loopback receiver

The hook callback client SHALL forward the agent's lifecycle payload unchanged to the loopback
hook receiver, so the existing hook-ingress contract continues to work without modification.

#### Scenario: Forward over a network bridge address

- **GIVEN** the agent invokes the client with a lifecycle payload on standard input and the
  bridge address, session id, and session token provided via the environment
- **WHEN** the client runs
- **THEN** it POSTs the payload verbatim to the configured bridge address
- **AND** it carries the session id and session token as request headers, exactly as the
  loopback receiver already expects

#### Scenario: Forward over a local control-channel path

- **GIVEN** the bridge address is a local control-channel path
- **WHEN** the client forwards the payload
- **THEN** it delivers over that local channel without opening a network port

#### Scenario: Bridge address absent

- **GIVEN** the bridge address is absent from the environment
- **WHEN** the client runs
- **THEN** it exits without error and forwards nothing

### Requirement: Fire-and-forget delivery never blocks the agent

The hook callback client SHALL never block or fail the agent, so a slow or unavailable receiver
cannot stall the session.

#### Scenario: Receiver slow, unreachable, or erroring

- **GIVEN** the receiver is slow, unreachable, or returns an error
- **WHEN** the client forwards a payload
- **THEN** it bounds its own runtime, suppresses its own errors, and always exits successfully
- **AND** the agent's hook step is never delayed beyond a short bound nor failed

### Requirement: Host resolves the client at a stable location

The host SHALL resolve the hook callback command from a single stable in-repo location, so hook
installation registers a path that exists after build.

#### Scenario: Host prepares the hook command at startup

- **GIVEN** the host prepares the hook callback command at startup
- **WHEN** it resolves the client
- **THEN** it points the command at the standalone script's stable path
- **AND** it surfaces a typed error if the script is absent

## REMOVED Requirements

### Requirement: Runtime-based hook callback script

**Reason**: The runtime-based client required that runtime to be present and resolvable in the
agent's environment at every hook fire, coupling a frequently-spawned foreign-process leaf to
the host runtime and adding per-fire startup cost. It is replaced by the runtime-free standalone
client above, which preserves the same forwarding contract while removing the runtime dependency.

**Migration**: Hook installation now registers the committed `bin/athing-notify` shell script
directly (no `bun` prefix, no build step). Existing installs are updated on the next
`engine.start()` via the non-destructive hook merge; the adapter's install/uninstall idempotency
marker moves from `notify.mjs` to `athing-notify`.
