## ADDED Requirements

### Requirement: Session-scoped first-party operations

The gateway SHALL expose first-party view, review, and diff operations through its existing standard MCP face. Each operation SHALL be authorized by the existing per-session token boundary and SHALL be limited to the session named by that verified token. Project authority SHALL be derived from that verified session, and an operation that names a different project SHALL be rejected. Rendered content SHALL receive a host-mediated bridge endpoint only; it SHALL NOT receive a session token, an MCP client, or authority to operate on another session. The first-party operations SHALL NOT alter gateway transport, service lifecycle, wire framing, ACL semantics, or management operations.

#### Scenario: Token targets a different session

- **WHEN** a first-party operation names a session other than the one authorized by its verified token
- **THEN** the gateway rejects the operation with a typed authorization error

#### Scenario: Operation targets another project's repository

- **WHEN** a first-party operation names a project other than the project derived from its verified session
- **THEN** the gateway rejects the operation with a typed authorization error without reading that project's repository

#### Scenario: Rendered content uses the bridge

- **WHEN** a rendered view requests diff data for its active session
- **THEN** the host performs the authorized operation without disclosing the session token to the view

### Requirement: Bounded first-party operation results

The gateway SHALL validate first-party operation inputs and SHALL enforce bounded request sizes, result sizes, and execution time. It SHALL return typed errors for invalid, unavailable, oversized, cancelled, or unauthorized operations. A failure of a first-party operation SHALL NOT interrupt terminal surfaces, session execution, or backend aggregation.

#### Scenario: Diff operation exceeds its result bound

- **WHEN** an authorized diff operation would exceed its configured result bound
- **THEN** the gateway returns a typed oversized-result error without returning partial authority or unbounded content
