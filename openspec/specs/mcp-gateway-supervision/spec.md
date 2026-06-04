# mcp-gateway-supervision Specification

## Purpose
Backend process lifecycle: the state model, eager and lazy spawn, health-watch, active-liveness healing, capped-backoff restart to a terminal Failed state, and idle-shutdown.

## Requirements

### Requirement: Backend state model

The gateway SHALL track each backend through a defined lifecycle: Disabled, Idle (configured but not
spawned), Starting, Ready, Unhealthy, Restarting, and Failed (restart attempts exhausted). State
transitions SHALL be observable.

#### Scenario: Reaches Ready after connect

- **WHEN** a backend is spawned and completes its handshake and initial indexing
- **THEN** its state SHALL be Ready

#### Scenario: Terminal Failed state

- **WHEN** a backend exhausts its restart budget
- **THEN** its state SHALL be Failed and the gateway SHALL stop attempting to restart it

### Requirement: Eager and lazy spawn

A non-lazy backend SHALL be spawned and indexed at startup. A lazy backend SHALL be indexed once so
its primitives are known, then shut down to release its process, and respawned on first routed call.
The gateway SHALL respect a backend's own list-changed capability as a hint that it is unsuitable for
lazy operation.

#### Scenario: Non-lazy spawned at startup

- **WHEN** the gateway starts and a backend is not lazy
- **THEN** that backend SHALL be spawned and indexed at startup and kept warm

#### Scenario: Lazy released after indexing

- **WHEN** a lazy backend has been indexed at startup
- **THEN** the gateway SHALL shut its process down while retaining its indexed primitives

#### Scenario: Lazy respawned on demand

- **WHEN** a routed call targets a lazy backend that is not running
- **THEN** the gateway SHALL spawn it, wait for its handshake before applying the call timeout, then
  run the call

#### Scenario: Re-index on respawn

- **WHEN** a lazy backend is respawned and its primitives differ from the indexed set
- **THEN** the gateway SHALL re-index it and notify the front client of the change

### Requirement: Crash and hang healing

The gateway SHALL detect a backend that exits and SHALL detect a backend that stops responding via
periodic liveness checks against warm backends. On either condition it SHALL transition the backend
to Unhealthy and restart it, subject to the restart budget.

#### Scenario: Exit detected and restarted

- **WHEN** a warm backend's connection ends
- **THEN** the gateway SHALL drop it from the index and restart it

#### Scenario: Hang detected and restarted

- **WHEN** a warm backend fails to answer a liveness check within the timeout
- **THEN** the gateway SHALL mark it Unhealthy and restart it

### Requirement: Capped-backoff restart

Restart attempts SHALL use exponential backoff with a configured ceiling. After a bounded number of
consecutive failures the backend SHALL enter the Failed state and no further automatic restarts SHALL
occur until explicitly requested.

#### Scenario: Backoff grows then caps

- **WHEN** a backend fails to start repeatedly
- **THEN** successive restart delays SHALL increase and SHALL NOT exceed the configured ceiling

#### Scenario: Stops after budget exhausted

- **WHEN** the consecutive-failure budget is exhausted
- **THEN** the gateway SHALL stop automatic restarts and mark the backend Failed

### Requirement: Idle shutdown

A lazy backend that has been warm without activity for a configured idle period SHALL be shut down
while its indexed primitives are retained, and SHALL be respawned on the next routed call.

#### Scenario: Idle backend released

- **WHEN** a lazy backend has had no routed calls for the idle period
- **THEN** the gateway SHALL shut its process down and retain its indexed primitives

### Requirement: Graceful shutdown

On daemon shutdown the gateway SHALL cancel all backend connections and allow their cleanup to
complete.

#### Scenario: All backends cancelled

- **WHEN** the daemon is shutting down
- **THEN** the gateway SHALL cancel every backend connection and wait for cleanup
