# hook-ingress

## Purpose

Defines how the engine installs agent lifecycle hooks, receives hook callbacks, authenticates them, and routes normalized `HookEvent` values to the correct session. The receiver mechanism is generic; all agent-specific knowledge lives in the adapter.

## Requirements

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

### Requirement: Authenticated callbacks

The receiver SHALL verify the per-session token on every callback and SHALL reject callbacks with a missing or mismatched token.

#### Scenario: Reject spoofed callback

- **WHEN** a callback arrives without the correct per-session token
- **THEN** the receiver SHALL reject it and SHALL NOT change any session state

### Requirement: Parse via the adapter and route by session id

For each authenticated callback the engine SHALL validate the envelope, call the adapter's parse function to produce a normalized `HookEvent`, and route it to the owning session by session id before dispatching it to the status and content consumers.

#### Scenario: Raw payload becomes a HookEvent

- **WHEN** an authenticated callback carries a raw agent payload
- **THEN** the engine SHALL call the adapter's parse function to produce a `HookEvent` and dispatch it to the owning session

#### Scenario: Unknown session

- **WHEN** a callback's session id matches no live session
- **THEN** the engine SHALL drop it without error to any other session

### Requirement: Idempotent dispatch

Because callbacks may be delivered more than once, applying a `HookEvent` SHALL be idempotent so duplicate callbacks do not corrupt session state.

#### Scenario: Duplicate callback

- **WHEN** the same lifecycle callback is received twice
- **THEN** the resulting session state SHALL be the same as if it were received once

### Requirement: Transport-agnostic boundary

The receiver mechanism (binding, auth, routing) SHALL be generic engine code; the agent-specific knowledge SHALL be limited to the adapter's hook-install data and parse function. Any other producer that emits a valid `HookEvent` SHALL be able to drive the engine through the same dispatch path.

#### Scenario: Alternate producer

- **WHEN** a producer other than the HTTP receiver dispatches a valid `HookEvent`
- **THEN** the engine SHALL process it identically, with no change to status or content logic
