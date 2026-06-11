## ADDED Requirements

### Requirement: SDK is a typed client of the orchestrator API

The SDK SHALL provide a typed client that invokes the orchestrator's request/response methods and
subscribes to its outbound event streams over the host transport. The client SHALL carry no backend
logic of its own; it SHALL only call the orchestrator API and surface typed results and events.

#### Scenario: Client invokes a request/response method

- **WHEN** a caller invokes a typed method on the SDK client
- **THEN** the call is routed to the orchestrator API over the host transport
- **AND** the typed result is returned to the caller

#### Scenario: Client subscribes to outbound events

- **WHEN** a caller subscribes through the SDK client
- **THEN** outbound events emitted by the orchestrator are delivered to the subscriber
- **AND** the client does not synthesize events of its own

### Requirement: Renderer reaches readiness through the client

The renderer SHALL reach a usable state by observing the orchestrator's `ready` state through the
SDK client. A blank renderer that reaches `ready` SHALL satisfy this requirement. The renderer SHALL
reflect a not-ready or boot-failure state when the orchestrator has not reached `ready`.

#### Scenario: Renderer observes ready

- **WHEN** the orchestrator reports `ready`
- **THEN** the renderer observes the `ready` state through the SDK client
- **AND** a blank renderer that reaches `ready` is acceptable

#### Scenario: Renderer reflects not-ready

- **WHEN** the orchestrator has not reached `ready`, or its boot has failed
- **THEN** the renderer reflects the not-ready or failure state rather than presenting as ready

### Requirement: In-renderer engine path is disabled

The renderer SHALL NOT drive an in-process agent engine. All backend interaction SHALL go through
the SDK client to the orchestrator API.

#### Scenario: No in-renderer engine on the desktop path

- **WHEN** the desktop application runs
- **THEN** no agent engine is constructed or driven inside the renderer
- **AND** backend interaction occurs only through the SDK client
