# desktop-daemon-host

## Purpose

The native core's ownership of the generic PTY daemon on the desktop path: spawning or adopting
and supervising it, performing the one-time startup bootstrap (resolve agent, verify version,
prepare hook command), shutting down an owned daemon gracefully on exit, and surfacing unexpected
daemon exits to the renderer.

## Requirements

### Requirement: Daemon sidecar spawned and supervised by the native core

The native core SHALL spawn the generic pseudo-terminal daemon as a supervised background
process on startup, reusing a live, compatible daemon when one is already recorded in the
ownership manifest, and SHALL NOT enable session interaction until the daemon is reachable.

#### Scenario: Starting the daemon on launch

- **WHEN** the application launches and no live, compatible daemon is recorded
- **THEN** the native core spawns the daemon and records its ownership
- **AND** it establishes a connection before enabling session interaction

#### Scenario: Adopting an existing daemon

- **WHEN** a live, compatible daemon is already recorded in the ownership manifest
- **THEN** the native core adopts it instead of spawning a duplicate

### Requirement: Startup bootstrap owned by the native core

The native core SHALL perform the one-time startup resolution the host owns — resolving the
agent executable, verifying its version, and preparing the hook callback command — and SHALL
make the resolved values available to the renderer for constructing the engine.

#### Scenario: Resolving startup values

- **WHEN** the application starts
- **THEN** the native core resolves the agent executable, verifies its version, and prepares the
  hook command
- **AND** it exposes those resolved values to the renderer

#### Scenario: Unsupported version is reported as a typed error

- **WHEN** the resolved agent version does not satisfy the supported range
- **THEN** the native core surfaces a typed version-unsupported error before accepting sessions

### Requirement: Ordered graceful shutdown of the owned daemon

The native core SHALL, on application exit, terminate the daemon it owns so the daemon can
complete its own shutdown, and SHALL leave an adopted daemon running.

#### Scenario: Shutting down an owned daemon

- **WHEN** the application exits and it spawned the daemon
- **THEN** the native core signals the daemon to shut down and waits for that sequence to begin

#### Scenario: Leaving an adopted daemon running

- **WHEN** the application exits and it adopted an already-running daemon
- **THEN** the native core leaves that daemon running

### Requirement: Detection of unexpected daemon exit

The native core SHALL detect when the supervised daemon exits unexpectedly and surface a typed
error to the renderer.

#### Scenario: Daemon crashes while running

- **WHEN** the supervised daemon exits unexpectedly while the application is running
- **THEN** the native core surfaces a typed lost-connection error to the renderer
