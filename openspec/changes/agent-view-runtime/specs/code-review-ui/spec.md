## ADDED Requirements

### Requirement: Durable local reviews and findings

The system SHALL persist reviews and findings as additive local operational state. A review SHALL retain its reviewed target, summary, walkthrough, and status. Each finding SHALL retain a project-relative file and range anchor, severity, category, body, state, and optional suggestion patch. The system SHALL validate that anchors and suggestions belong to the reviewed target before persistence. Review state SHALL survive app restart, while bridge endpoints, rendered content, capability grants, and dock selection SHALL be discarded when the view closes or the app exits. The persistence model SHALL NOT modify workspace snapshots, launch specifications, panel trees, surface bindings, or placement records.

#### Scenario: Review is reopened after restart

- **WHEN** the app restarts after a review and its findings were persisted
- **THEN** the review and its anchors are available as local operational state

#### Scenario: Finding anchor does not belong to the target

- **WHEN** a review publication includes a finding whose file or range does not belong to the reviewed target
- **THEN** the system rejects that finding with a typed validation error and does not persist it

### Requirement: User-controlled review resolution and suggestion application

The review UI SHALL render findings over the structured diff model and SHALL allow a user to filter, accept, or dismiss a finding. The UI SHALL show a stale or unresolved anchor as review state. Publishing a review or suggestion SHALL NOT write a file. Applying a suggestion patch SHALL require a separate explicit user action and SHALL recheck the reviewed target against the current working tree before writing; a stale, conflicting, or changed target SHALL produce a typed result and SHALL NOT be applied automatically.

#### Scenario: User applies a current suggestion

- **WHEN** the user explicitly applies a suggestion and the reviewed target still matches the current working tree
- **THEN** the system applies that suggestion and records the resulting finding state

#### Scenario: Suggestion becomes stale

- **WHEN** the user explicitly applies a suggestion after the reviewed target has changed
- **THEN** the system reports the stale target and does not apply the patch automatically
