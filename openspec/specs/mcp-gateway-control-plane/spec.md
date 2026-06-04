# mcp-gateway-control-plane Specification

## Purpose
The REST management surface: health and status endpoints, targeted restart/stop/start, and reload with per-backend diff and graceful drain.

## Requirements

### Requirement: Management surface separate from tools

The daemon SHALL expose management operations over a control-plane interface that is distinct from
the aggregated MCP tool surface, so that an agent calling tools cannot administer the daemon. The
control-plane SHALL share the same loopback server and access token as the MCP endpoint.

#### Scenario: Admin not exposed as a tool

- **WHEN** a client lists the aggregated tools
- **THEN** management operations such as restart and reload SHALL NOT appear as tools

#### Scenario: Control-plane requires the token

- **WHEN** a control-plane request other than health lacks the correct token
- **THEN** the daemon SHALL reject it as unauthorized

### Requirement: Health endpoint

The daemon SHALL expose an unauthenticated health endpoint returning a liveness status and the daemon
version, carrying no sensitive data, so a launcher can probe before holding the token.

#### Scenario: Health without token

- **WHEN** the health endpoint is requested without a token
- **THEN** the daemon SHALL respond with a liveness status and version

### Requirement: Backend status endpoints

The daemon SHALL expose endpoints listing all backends and a single backend's detail, reporting at
least name, state, process id when running, uptime, restart count, exposed tool count, and last error.

#### Scenario: List backends

- **WHEN** the backends list endpoint is requested with a valid token
- **THEN** the daemon SHALL return each backend's name, state, and summary fields

#### Scenario: Backend detail

- **WHEN** a single backend's detail endpoint is requested
- **THEN** the daemon SHALL return that backend's state and summary fields including last error

### Requirement: Targeted lifecycle control

The daemon SHALL expose endpoints to restart, stop, and start a single named backend without
affecting others. A targeted restart SHALL reset the backend's restart budget, respawn it, re-index
it, and notify the front client of any resulting primitive change.

#### Scenario: Restart one backend

- **WHEN** a restart is requested for a named backend
- **THEN** the daemon SHALL restart only that backend, reset its restart budget, re-index it, and
  notify the client if its primitives changed

#### Scenario: Stop and start one backend

- **WHEN** a stop is requested for a named backend and later a start
- **THEN** the daemon SHALL move it to Idle on stop and spawn it on start, leaving other backends
  untouched

### Requirement: Config reload with diff

The daemon SHALL expose a reload operation that re-reads and validates `mcp.json`. Invalid config
SHALL be rejected without disrupting running backends. Valid config SHALL be diffed against the
running set per backend: added backends SHALL be brought up, removed backends SHALL be taken down,
backends whose spawn-affecting fields (command, args, env, url, headers) changed SHALL be respawned,
and backends whose only policy fields (allowlist, lazy) changed SHALL be updated in place without
respawning. Reload SHALL be serialized so that concurrent reloads do not overlap, and SHALL return a
report listing added, removed, restarted, updated, unchanged, and failed backends. Per-backend
failures SHALL NOT roll back the other changes.

#### Scenario: Invalid config rejected safely

- **WHEN** reload reads a config that fails validation
- **THEN** the daemon SHALL reject the reload and SHALL leave running backends undisturbed

#### Scenario: Policy-only change avoids respawn

- **WHEN** a backend's only change is its allowlist or lazy flag
- **THEN** the daemon SHALL apply the change in place, re-filter its primitives, and SHALL NOT respawn
  the process

#### Scenario: Spawn-field change respawns

- **WHEN** a backend's command, args, env, url, or headers changed
- **THEN** the daemon SHALL respawn that backend

#### Scenario: Reload reports outcome

- **WHEN** a reload completes
- **THEN** the daemon SHALL return a report classifying each backend as added, removed, restarted,
  updated, unchanged, or failed

#### Scenario: Partial failure isolated

- **WHEN** one backend fails to come up during reload
- **THEN** the daemon SHALL still apply the other changes and report that backend as failed

### Requirement: Graceful drain on disruptive change

The daemon SHALL drain a backend that has in-flight calls when a restart, stop, removal, or respawn
would affect it: new calls to that backend SHALL block until the swap completes, and the daemon SHALL
wait for in-flight calls to finish up to a bounded timeout before proceeding, force-cancelling
remaining calls only after the timeout. Blocked new calls SHALL be served by the new instance once
ready.

#### Scenario: In-flight calls drained

- **WHEN** a disruptive change targets a backend with in-flight calls
- **THEN** the daemon SHALL wait for those calls to finish, up to the drain timeout, before swapping

#### Scenario: New calls park during swap

- **WHEN** a new call arrives for a backend that is draining
- **THEN** the daemon SHALL block the call until the new instance is ready and then serve it

#### Scenario: Force-cancel after timeout

- **WHEN** in-flight calls do not finish within the drain timeout
- **THEN** the daemon SHALL force-cancel the remaining calls and proceed with the change
