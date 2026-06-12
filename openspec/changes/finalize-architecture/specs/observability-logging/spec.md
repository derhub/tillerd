## ADDED Requirements

### Requirement: correlation_id propagates across process hops

Every operation SHALL be assigned a `correlation_id` at its ingress (desktop IPC command
or surface operation). The id SHALL be bound into the logger context in the originating
process and carried as a field on existing request envelopes across every process hop,
so all structured records for one operation join on the same `correlation_id` key in
every process.

#### Scenario: One operation joins across processes

- **WHEN** a desktop IPC command flows through the orchestrator to the daemon
- **THEN** structured records in the orchestrator and the daemon for that operation
  carry the same `correlation_id` value

#### Scenario: The key is part of the standardized vocabulary

- **WHEN** any runtime (TS or Rust) emits a record for a correlated operation
- **THEN** the attribute key is exactly `correlation_id`
