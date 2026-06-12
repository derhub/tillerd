## ADDED Requirements

### Requirement: open_agent opens an agent surface with gate registration

The surface runtime SHALL provide an `open_agent` operation that, given a `surface_id` and agent configuration, registers the surface with the gate, runs the hook-install routine, and spawns the agent process via the daemon — in that order. The runtime SHALL return a typed error if any step fails before spawn.

#### Scenario: Gate registration precedes spawn

- **WHEN** `open_agent` is called
- **THEN** the runtime SHALL register the surface with the gate before sending the daemon spawn command

#### Scenario: Hook install precedes spawn

- **WHEN** `open_agent` is called
- **THEN** the runtime SHALL run the hook-install routine before sending the daemon spawn command

#### Scenario: Typed error returned on gate registration failure

- **WHEN** gate registration fails during `open_agent`
- **THEN** the runtime SHALL return a typed error and SHALL NOT attempt to spawn the agent

### Requirement: SurfaceEventSink exposes on_content callback

The `SurfaceEventSink` trait SHALL include an `on_content` callback that the surface runtime calls when a content event is derived from a hook event for a given surface. The callback SHALL carry the `surface_id` and the `ContentEvent`.

#### Scenario: on_content called with correct surface_id

- **WHEN** a hook event for surface S produces a content event
- **THEN** `on_content` is called with S's `surface_id` and the derived `ContentEvent`

#### Scenario: on_content not called for events with no content

- **WHEN** a hook event produces no content event
- **THEN** `on_content` SHALL NOT be called for that event

### Requirement: SurfaceEventSink exposes on_error callback

The `SurfaceEventSink` trait SHALL include an `on_error` callback that the surface runtime calls when a non-recoverable surface-level error occurs (such as a gate subscription abort) after the surface was opened. The callback SHALL carry the `surface_id` and a descriptive error string.

#### Scenario: on_error called on gate subscription abort

- **WHEN** the gate sends an error frame for a surface's subscription after it was established
- **THEN** `on_error` is called with that surface's `surface_id` and the error description

## MODIFIED Requirements

### Requirement: Detach preserves the pseudo-terminal; removal terminates it

A proxy detach caused by host shutdown or a dropped client SHALL leave the pseudo-terminal running
in the daemon so the surface can resume; the pseudo-terminal's lifetime SHALL follow the surface,
not the client connection. Removing a terminal surface SHALL terminate its pseudo-terminal and
release the proxy. Removing an agent surface SHALL additionally cancel its gate subscription and run
the hook-uninstall routine before releasing the proxy.

#### Scenario: Detach keeps the pseudo-terminal alive

- **WHEN** the host shuts down or a client disconnects
- **THEN** the proxy detaches and the pseudo-terminal keeps running in the daemon

#### Scenario: Removal terminates the pseudo-terminal

- **WHEN** the surface is removed
- **THEN** its pseudo-terminal is terminated and the proxy is released

#### Scenario: Agent surface removal also cancels gate subscription

- **WHEN** an agent surface is removed
- **THEN** the gate subscription for that `surface_id` is cancelled before the proxy is released

#### Scenario: Agent surface removal also runs hook uninstall

- **WHEN** an agent surface is removed
- **THEN** the hook-uninstall routine is run for that surface's agent home
