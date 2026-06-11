## ADDED Requirements

### Requirement: Gate subscription opened per agent surface

When an agent surface is opened, the surface runtime SHALL open a subscription to the gate's fan-out channel keyed by the surface's `surface_id`. The subscription SHALL be established before the agent process is spawned so that no hook events are missed.

#### Scenario: Subscription opens before spawn

- **WHEN** `open_agent` is called for a surface
- **THEN** the runtime SHALL open the gate subscription for that `surface_id` before issuing the daemon spawn command

#### Scenario: Subscription established with correct surface_id

- **WHEN** the gate subscription is opened
- **THEN** it SHALL use the surface's `surface_id` as the gate session id so that only hook events correlated to that surface are received

### Requirement: Hook events decoded and routed to parse functions

For each hook event received over the gate subscription, the surface runtime SHALL call the Rust parse functions to derive an `AgentStatus` and an optional `ContentEvent`, then emit them over the `SurfaceEventSink`.

#### Scenario: Status event emitted for every hook event

- **WHEN** a hook event arrives for an agent surface
- **THEN** the runtime SHALL call the status-mapping function and emit the resulting `AgentStatus` via `SurfaceEventSink::on_status`

#### Scenario: Content event emitted when present

- **WHEN** a hook event maps to a non-None `ContentEvent`
- **THEN** the runtime SHALL emit the content event via `SurfaceEventSink::on_content`

#### Scenario: No content call for events with no content

- **WHEN** a hook event maps to `None` content
- **THEN** the runtime SHALL NOT call `SurfaceEventSink::on_content`

### Requirement: Gate subscription torn down on surface remove

When an agent surface is removed, the surface runtime SHALL cancel its gate subscription for that `surface_id` and release associated resources.

#### Scenario: Subscription cancelled on remove

- **WHEN** `remove` is called for an agent surface
- **THEN** the runtime SHALL stop receiving hook events for that `surface_id` and close the gate connection

#### Scenario: Subscription is idempotent on duplicate remove

- **WHEN** `remove` is called more than once for the same surface
- **THEN** the runtime SHALL NOT panic or return an error for the second call

### Requirement: Typed error on gate subscription failure

If the gate subscription cannot be established or is aborted by the gate, the surface runtime SHALL emit a typed error through `SurfaceEventSink` rather than silently dropping events.

#### Scenario: Gate unavailable on open

- **WHEN** the gate is not reachable when `open_agent` is called
- **THEN** the runtime SHALL return a typed `Error` from `open_agent`

#### Scenario: Subscription aborted mid-session

- **WHEN** the gate sends an error frame after the subscription was established
- **THEN** the runtime SHALL emit the error via `SurfaceEventSink::on_error` and stop routing events for that surface
