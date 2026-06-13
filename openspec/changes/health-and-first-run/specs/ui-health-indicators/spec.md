## ADDED Requirements

### Requirement: A single aggregate health indicator is shown as read-only chrome

The interface SHALL display one aggregate health indicator in the application
shell whose state reflects the worst current state across all observed services
(and the orchestrator boot state).

#### Scenario: Aggregate reflects the worst service state

- **WHEN** all observed services are available
- **THEN** the aggregate indicator shows a healthy state
- **AND WHEN** any observed service is unavailable or failed
- **THEN** the aggregate indicator shows a failed state

#### Scenario: Starting state while services come up

- **WHEN** any observed service has not yet reached availability and none has failed
- **THEN** the aggregate indicator shows a starting state

### Requirement: Clicking the indicator opens a panel listing every service

Activating the indicator SHALL open a dismissible, non-modal panel that lists one
row per observed service.

#### Scenario: Panel lists each service

- **WHEN** the user activates the aggregate indicator
- **THEN** a panel opens showing one row per observed service with its name and current state

#### Scenario: Panel is dismissible and non-blocking

- **WHEN** the panel is open and the user clicks outside it or dismisses it
- **THEN** the panel closes
- **AND** the application shell was never blocked or dimmed while it was open

### Requirement: Each service row shows detail on demand

Each row in the panel SHALL show the service version and state, the failure reason
when the service has failed, and a way to open the logs viewer filtered to that
service.

#### Scenario: Row reveals version and state

- **WHEN** the panel is open
- **THEN** each row shows its service's version and state
- **AND WHEN** a service has failed
- **THEN** its row shows the failure reason

#### Scenario: Row links to that service's logs

- **WHEN** the user follows a row's logs link
- **THEN** the logs viewer opens pre-filtered to that service's records

### Requirement: Version-mismatch is surfaced through the indicator

A service running an unexpected version or draining SHALL be surfaced through the
aggregate indicator and its row, not through a separate screen.

#### Scenario: Version mismatch shown inline

- **WHEN** a service reports a version-mismatch or draining state
- **THEN** the aggregate indicator reflects a non-healthy state
- **AND** that service's row explains the version-mismatch or draining state
- **AND** no separate blocking screen is shown

### Requirement: The indicator and panel are observation only

The indicator and its panel SHALL NOT offer controls that start, stop, or restart
a service.

#### Scenario: No lifecycle controls present

- **WHEN** the user views the indicator and opens its panel
- **THEN** no control to change any service's lifecycle is present
