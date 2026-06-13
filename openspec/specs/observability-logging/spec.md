# observability-logging Specification

## Purpose

Defines the structured-logging contract shared by the TypeScript and Rust runtimes — context binding via child loggers, per-process resource identity, OpenTelemetry-mappable JSON output, and a standardized dotted attribute vocabulary — while keeping core packages logging-library-agnostic.

## Requirements

### Requirement: Logger supports context binding

The `Logger` interface SHALL provide a `child(context)` operation that returns a new `Logger`
carrying the given structured context. Every record emitted by a child logger MUST include the
bound context fields without the caller re-supplying them. Child loggers MUST compose: a child
of a child MUST carry the merged context of both, with the inner binding taking precedence on
key collision.

#### Scenario: Bound context inherited by every record

- **WHEN** a child logger is created with context `{ "session.id": "s1", "pty.pid": 42 }` and
  `info("spawning pty", { "binary": "claude" })` is called on it
- **THEN** the emitted record contains `session.id = "s1"`, `pty.pid = 42`, and
  `binary = "claude"` without the call site passing `session.id` or `pty.pid`

#### Scenario: Children compose

- **WHEN** `root.child({ "component": "daemon" }).child({ "session.id": "s1" })` emits a record
- **THEN** that record contains both `component = "daemon"` and `session.id = "s1"`

#### Scenario: Inner binding wins on collision

- **WHEN** a child binds `{ "component": "daemon" }` and its grandchild binds
  `{ "component": "pty" }` and the grandchild emits a record
- **THEN** the record's `component` value is `"pty"`

### Requirement: Per-process resource identity

A logger SHALL be constructed with a resource describing the emitting process. The resource MUST
include `service.name` and `service.version`, and MAY include `service.instance.id`,
`host.name`, and `process.pid`. Every record emitted by that logger and its children MUST carry
the resource fields. Construction MUST NOT take a privileged single correlation parameter such
as `sessionId`; session correlation is bound as ordinary context via `child`.

#### Scenario: Resource stamped on every record

- **WHEN** a logger is constructed with resource `{ "service.name": "tillerd-daemon",
"service.version": "0.1.0" }` and emits any record
- **THEN** the record contains `service.name = "tillerd-daemon"` and
  `service.version = "0.1.0"`

#### Scenario: Resource inherited through children

- **WHEN** a child of a resource-bearing logger emits a record
- **THEN** the record still carries the parent resource fields

### Requirement: Structured JSON output mappable to the OpenTelemetry log model

Loggers SHALL emit one structured JSON record per line containing, at minimum, a timestamp, a
severity level, a message body, the bound attributes, and the resource fields. The field
semantics MUST map to the OpenTelemetry log data model (timestamp, severity, body, attributes,
resource) so a downstream collector can ingest records without transformation logic beyond
field renaming. The system SHALL NOT require any exporter, collector, or external pipeline to
be running for logging to function.

#### Scenario: One JSON record per line

- **WHEN** any log call is made
- **THEN** exactly one valid JSON object is written on its own line, carrying timestamp,
  severity, body, attributes, and resource fields

#### Scenario: Logging functions with no collector present

- **WHEN** no OpenTelemetry collector or exporter is configured or running
- **THEN** records are still written to the configured destination and no error is raised

### Requirement: Standardized attribute key vocabulary

Both runtimes SHALL use a single, dotted attribute-key vocabulary for correlation. Resource keys
are `service.name`, `service.version`, `service.instance.id`, `host.name`, `process.pid`.
Attribute keys for runtime correlation include `session.id`, `pty.pid`, `hook.event`,
`component`, and `frame.seq`. The same concept MUST use the same key in both the TypeScript and
Rust runtimes.

#### Scenario: Same key across runtimes

- **WHEN** the TypeScript logger and the native daemon both record a session correlation field
- **THEN** both use the key `session.id` (not `sessionId` or `session_id`)

### Requirement: Native daemon emits structured context-bound logs

The native PTY daemon (`daemon-pty`) SHALL emit leveled, structured, context-bound log records
rather than unstructured `eprintln!` text. Each record MUST carry the process resource and,
where a record pertains to a session, the `session.id` attribute. Output MUST be JSON conforming
to the same field semantics as the TypeScript logger.

#### Scenario: Session-scoped daemon record carries correlation

- **WHEN** the native daemon logs an event while handling a known session (e.g. a PTY spawn or
  exit)
- **THEN** the emitted JSON record carries `session.id`, the daemon resource
  (`service.name`, `service.version`), and a severity level

#### Scenario: No unstructured stderr text remains

- **WHEN** the native daemon reports any operational event previously written via `eprintln!`
- **THEN** that event is emitted as a structured JSON log record at an appropriate level, not as
  plain stderr text

### Requirement: Core packages remain logging-library-agnostic

The `@tillerd/sdk` and `@tillerd/engine` packages SHALL depend only on the injected `Logger`
interface and MUST NOT import any concrete logging library. The context-binding additions MUST
preserve the inward-pointing dependency rule so the core remains runtime-neutral.

#### Scenario: Engine uses only the injected interface

- **WHEN** the engine emits logs and binds context
- **THEN** it does so exclusively through the injected `Logger` interface (including `child`)
  and imports no logging implementation

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
