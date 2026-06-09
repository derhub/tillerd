## Purpose

Defines the standard interface every long-lived service must implement and the single entrypoint through which all services are started. Establishing a uniform contract eliminates per-service boilerplate, provides a stable extension point for shared capabilities (health being the first), and makes liveness observable across services without any per-service socket or protocol.

## Requirements

### Requirement: A service is defined by one standard interface

A long-lived service SHALL be defined by implementing a single standard interface comprising its identity and version, a serve behavior run until stop, a teardown behavior on stop, and a health self-check. Creating a service SHALL require only implementing that interface and starting it through the one standard entrypoint; the entrypoint SHALL set up the runtime, run the service under the host, and report a startup failure uniformly, so every service is created the same way with no per-service entrypoint boilerplate.

#### Scenario: Implementing the interface is sufficient to create a service

- **WHEN** a new service implements the standard interface and is started through the standard entrypoint
- **THEN** it SHALL run with manifest publication, signal handling, and graceful shutdown all provided by the host, without the service re-implementing them

#### Scenario: Entrypoints do not diverge

- **WHEN** two services are started
- **THEN** each SHALL use the same single entrypoint call, so their startup paths cannot drift

### Requirement: The contract is the extension point

The standard interface SHALL be the single place new shared service capabilities are added, as methods with defaults so adding one does not break existing services. Health is the first such capability.

#### Scenario: A new capability is added without breaking services

- **WHEN** a new shared capability is added to the standard interface as a defaulted method
- **THEN** existing services SHALL continue to compile and run unchanged, inheriting the default

### Requirement: Health is an in-process self-check

The standard interface SHALL include a health self-check returning at least the service's liveness status and version. It SHALL be an in-process method the service answers itself — there SHALL be no health socket and no health request protocol. A default SHALL report the service serving at its configured version, so a service need not implement it; a service MAY override it to report a different status, such as draining.

#### Scenario: Default health reports serving and version

- **WHEN** a service that does not override health is asked for its health
- **THEN** the report SHALL indicate a serving status and the service's configured version

#### Scenario: A service reports its own status

- **WHEN** a service that is shutting down is asked for its health
- **THEN** it MAY report a draining (non-serving) status determined by its own logic

#### Scenario: No health socket exists

- **WHEN** a service is running
- **THEN** the runtime directory SHALL contain no dedicated health socket; health is answered in-process, not over a socket

### Requirement: The host surfaces health uniformly

The host SHALL obtain a service's health by calling its in-process self-check and SHALL surface it uniformly — at minimum logging the status at startup and during graceful drain — so health is observable across services without any per-service health endpoint.

#### Scenario: Health is logged at startup and drain

- **WHEN** a service starts, and again when it begins graceful shutdown
- **THEN** the host SHALL call the service's health self-check and log the reported status

### Requirement: Liveness stays externally checkable without a dedicated socket

Removing the dedicated health socket SHALL NOT change how a launcher checks a running instance: version SHALL be read from the manifest and liveness from a connect to the main control socket. Adopt-or-spawn behavior SHALL be unchanged.

#### Scenario: Adopt-or-spawn is unchanged

- **WHEN** a launcher decides whether to adopt a running instance
- **THEN** it SHALL read the version from the manifest and liveness from a connect to the main control socket, with no dependency on the removed health socket
