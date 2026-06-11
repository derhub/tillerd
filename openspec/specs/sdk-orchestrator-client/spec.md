# sdk-orchestrator-client

## Purpose

The SDK's typed client of the orchestrator API: it invokes the orchestrator's request/response
methods and subscribes to its outbound event streams over the host transport, carries no backend
logic of its own, and is how the renderer reaches readiness — with the in-renderer agent engine
path disabled.
## Requirements
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

### Requirement: Typed terminal-surface client

The SDK SHALL provide a typed client for terminal surfaces over the orchestrator API: create a
terminal surface in a session, subscribe to a surface's raw output byte stream and its
terminal-status stream, send input, and send resize. The client SHALL route by surface identifier
and SHALL NOT connect to the daemon directly.

#### Scenario: Create a terminal surface through the SDK

- **WHEN** a consumer creates a terminal surface through the SDK
- **THEN** the SDK calls the orchestrator API and returns the surface identifier

#### Scenario: Subscribe to bytes and status

- **WHEN** a consumer subscribes to a surface
- **THEN** the SDK delivers the surface's raw output bytes and terminal-status changes as they arrive over the orchestrator event stream

#### Scenario: Send input and resize

- **WHEN** a consumer sends input or a resize for a surface
- **THEN** the SDK forwards it to the orchestrator keyed by the surface identifier
- **AND** it does not open a connection to the daemon

