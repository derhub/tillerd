## ADDED Requirements

### Requirement: Every long-lived service writes structured JSON logs to a rolling per-service file

Every long-lived service SHALL write its structured JSON log records to a rolling, per-service
file under the runtime logs directory (`<runtime>/logs/<service>.<date>.log`), rotated daily.
This covers the PTY daemon, the gate, the MCP gateway, and the orchestrator host. No
long-lived service SHALL emit operational logs as unstructured text to stderr or stdout.
Records MUST conform to the JSON field semantics already defined for structured output
(timestamp, severity, body, attributes, resource).

#### Scenario: Service logs to a rolling per-service file

- **WHEN** a long-lived service emits a log record
- **THEN** the record is written as one JSON line to `<runtime>/logs/<service>.<date>.log`,
  rotated daily

#### Scenario: MCP gateway emits structured file logs, not plain stderr

- **WHEN** the MCP gateway reports an operational event
- **THEN** the event is a structured JSON record in the gateway's rolling log file, not plain
  text on stderr

#### Scenario: Resource identifies the emitting service

- **WHEN** any long-lived service writes a record to its file
- **THEN** the record carries `service.name` and `service.version` for that service
