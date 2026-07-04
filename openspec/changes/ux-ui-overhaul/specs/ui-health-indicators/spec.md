# ui-health-indicators

## MODIFIED Requirements

### Requirement: A single aggregate health indicator is shown as read-only chrome

The interface SHALL display one aggregate health indicator as a status bar item whose
state reflects the worst current state across all observed services (and the orchestrator
boot state).

#### Scenario: Aggregate reflects the worst service state

- **WHEN** all observed services are available
- **THEN** the aggregate indicator shows a healthy state
- **AND WHEN** any observed service is unavailable or failed
- **THEN** the aggregate indicator shows a failed state

#### Scenario: Starting state while services come up

- **WHEN** any observed service has not yet reached availability and none has failed
- **THEN** the aggregate indicator shows a starting state
