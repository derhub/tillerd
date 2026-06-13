## ADDED Requirements

### Requirement: Per-service status is observable

The system SHALL expose, for each supervised service, its name, its version, and a
state drawn from a defined set: starting, ready, draining, version-mismatch, and
unavailable.

#### Scenario: Each supervised service is reported

- **WHEN** the orchestrator has supervised its services
- **THEN** each supervised service appears in the observable status with its name, version, and state

#### Scenario: Each state is distinguishable

- **WHEN** a supervised service is available at the expected version
- **THEN** its state reads as ready
- **AND WHEN** a supervised service is not available
- **THEN** its state reads as unavailable
- **AND WHEN** a supervised service has not yet reached availability
- **THEN** its state reads as starting

#### Scenario: Version mismatch and draining are distinct from ready

- **WHEN** a supervised service is running a version other than the expected version
- **THEN** its state reads as version-mismatch, distinct from ready
- **AND WHEN** a supervised service is winding down
- **THEN** its state reads as draining, distinct from ready

### Requirement: Health is derived from the existing discovery record

The system SHALL derive per-service status from the existing service discovery
record and SHALL NOT add any health probe, socket, or route to a service.

#### Scenario: Status comes from discovery, not a probe

- **WHEN** per-service status is produced
- **THEN** it is read from each service's existing discovery record
- **AND** no additional health endpoint is opened on any service

### Requirement: A host-agnostic source delivers status to the interface

The per-service status SHALL be delivered to the user interface through a
host-agnostic source so an alternative host can satisfy the same shape without
changing consumers.

#### Scenario: Desktop host provides the source

- **WHEN** the interface runs on the desktop host
- **THEN** a desktop source provides the per-service status

#### Scenario: Source absent on an unsupported host

- **WHEN** the interface runs on a host that provides no source
- **THEN** the source resolves as absent and consumers degrade gracefully rather than erroring

### Requirement: Per-service health is read-only

The per-service health source SHALL expose observation only and SHALL NOT expose
any operation that starts, stops, or restarts a service.

#### Scenario: No mutating operation is exposed

- **WHEN** a consumer holds the health source
- **THEN** it can read status
- **AND** it has no operation to change a service's lifecycle
