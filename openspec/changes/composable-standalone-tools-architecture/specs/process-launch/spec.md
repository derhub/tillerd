## ADDED Requirements

### Requirement: Adopt-or-spawn a tool

Given a tool's identity and manifest, the launcher SHALL connect to a live, matching-version
instance if one is running, and otherwise SHALL spawn one and wait until it is reachable before
returning. A stale manifest naming a dead instance SHALL be overwritten by the spawn.

#### Scenario: Adopt a live matching instance

- **WHEN** a matching-version instance is already running
- **THEN** the launcher SHALL connect to it and SHALL NOT start a second instance

#### Scenario: Spawn when none is running

- **WHEN** no live instance is found
- **THEN** the launcher SHALL spawn one and SHALL return only once it is reachable

#### Scenario: Stale manifest overwritten

- **WHEN** the manifest names an instance that is no longer alive
- **THEN** the launcher SHALL proceed to spawn and overwrite the stale manifest

### Requirement: Spawn-field diffing

The launcher SHALL decide whether a managed child needs restarting by comparing only the fields
that affect the spawned process. A change to a field that does not affect the process SHALL NOT
force a restart.

#### Scenario: Spawn-affecting change triggers restart

- **WHEN** a field that affects the spawned process changes
- **THEN** the launcher SHALL treat the child as needing a restart

#### Scenario: Non-spawn change does not restart

- **WHEN** only a field that does not affect the spawned process changes
- **THEN** the launcher SHALL NOT restart the child

### Requirement: Bounded-backoff restart

The launcher SHALL restart a managed child that exits, using a capped backoff so a persistently
failing child does not spin.

#### Scenario: Capped backoff on repeated failure

- **WHEN** a managed child exits repeatedly
- **THEN** the launcher SHALL restart it with a backoff that does not exceed a fixed cap
