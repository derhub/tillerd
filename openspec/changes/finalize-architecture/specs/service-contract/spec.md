## ADDED Requirements

### Requirement: Ready is a first-class lifecycle phase

A service SHALL signal readiness through the host once its serve behavior is accepting
work (socket listening). The host SHALL record the transition in the service manifest
(`status: starting -> ready`) and log it. Consumers SHALL read readiness from the
manifest, not infer it from socket connects.

#### Scenario: Service signals ready

- **WHEN** a service calls the ready handle on its serve context
- **THEN** the manifest status changes from `starting` to `ready`
- **AND** the host logs the transition

#### Scenario: Spawned but not yet ready

- **WHEN** a service has been started but has not signaled ready
- **THEN** the manifest status is `starting` and consumers treat it as not yet available

### Requirement: Drain is a first-class lifecycle phase

The host SHALL translate a drain signal (SIGUSR2) into the service's drain phase: the
service refuses new work, lets active work finish, reports `Draining` health, and the
manifest status becomes `draining`. The host SHALL exit cleanly once the service is
idle. SIGTERM keeps its existing graceful-shutdown-now meaning.

#### Scenario: Drain signal flips the service to draining

- **WHEN** a running service receives SIGUSR2
- **THEN** its health reports `Draining` and the manifest status becomes `draining`

#### Scenario: Draining service refuses new work

- **WHEN** a client requests new work from a draining service
- **THEN** the request is refused with a typed error while active work continues

#### Scenario: Drained service exits when idle

- **WHEN** the last active work item on a draining service completes
- **THEN** the host shuts down cleanly and removes the manifest

### Requirement: Discovery is resolved from the manifest

A service manifest SHALL carry name, version, pid, lifecycle status, and the service's
socket path under the runtime layout. Clients SHALL resolve a service's socket from its
manifest only.

#### Scenario: Client resolves a service socket

- **WHEN** a client needs to reach a service
- **THEN** it reads the socket path from that service's manifest and connects to it
