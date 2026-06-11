# orchestrator-supervision

## Purpose

How the orchestrator ensures its shared services — the gate and the daemon — are running at boot
by adopting a live compatible instance or spawning one, tracks each supervised service's status
through its in-process health self-check, and gates `ready` on every supervised service being
available.

## Requirements

### Requirement: Adopt-or-spawn shared services at boot

On boot the orchestrator SHALL ensure each shared service it depends on — the gate and the
daemon — is running, by adopting an already-running instance when one is live and compatible, and
otherwise spawning it, through the standard service contract. Liveness SHALL be determined by
connecting to the service's main control socket, and version SHALL be read from the service's
manifest.

#### Scenario: Adopt a live compatible service

- **WHEN** a shared service is already running and its manifest version is compatible
- **THEN** the orchestrator adopts the running instance
- **AND** it does not spawn a duplicate

#### Scenario: Spawn an absent service

- **WHEN** a shared service is not running
- **THEN** the orchestrator spawns it through the service contract
- **AND** then treats it as a supervised service

### Requirement: Per-service status via in-process health

The orchestrator SHALL track the status of each supervised service using that service's
in-process health self-check, without a dedicated health socket. The tracked status SHALL include
at least liveness and version, sourced from a control-socket connect and the service manifest.

#### Scenario: Healthy service reported available

- **WHEN** a supervised service is live and reports a healthy self-check
- **THEN** the orchestrator records the service as available with its version

#### Scenario: Unavailable service reported not available

- **WHEN** a supervised service cannot be reached on its control socket
- **THEN** the orchestrator records the service as not available

### Requirement: Readiness is gated on supervised services

The orchestrator SHALL NOT reach `ready` until every supervised service has been adopted or
spawned and reports available. If a required service cannot be made available, the orchestrator
SHALL surface a typed failure rather than reporting a false `ready`.

#### Scenario: Ready only after all services are available

- **WHEN** both the gate and the daemon are adopted or spawned and report available
- **THEN** the orchestrator may reach `ready`

#### Scenario: Required service cannot be made available

- **WHEN** a required service can neither be adopted nor spawned to an available state
- **THEN** the orchestrator surfaces a typed failure
- **AND** it does not report `ready`
