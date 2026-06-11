## MODIFIED Requirements

### Requirement: Session-scoped terminal connection

The terminal pane SHALL accept the identifier of a session's terminal surface as input and attach to
that surface's output stream through the orchestrator client. When the surface identifier changes,
the previous attachment SHALL be torn down and a new one established.

#### Scenario: Connection opens for surface

- **WHEN** the terminal pane mounts with a given surface identifier
- **THEN** it attaches to that surface through the orchestrator client and begins streaming output

#### Scenario: Reattach on surface change

- **WHEN** the active surface identifier changes (e.g. the user navigates to a different session)
- **THEN** the prior attachment is closed, the terminal buffer is cleared, and a new attachment opens for the new surface identifier

#### Scenario: Teardown on unmount

- **WHEN** the terminal pane unmounts
- **THEN** the attachment is closed cleanly and the surface's pseudo-terminal keeps running for later resume

### Requirement: Terminal output rendering

The terminal pane SHALL render surface output as it arrives, preserving ANSI escape sequences and
raw bytes. The pane SHALL auto-fit to its container's dimensions and send a resize for the surface to
the orchestrator through the client when the container size changes.

#### Scenario: Output streams to display

- **WHEN** the orchestrator delivers output bytes for the surface
- **THEN** the terminal renders them immediately including colors, cursor movement, and other control sequences

#### Scenario: Resize propagates

- **WHEN** the terminal container is resized (window resize or panel drag)
- **THEN** the pane recalculates dimensions and sends a resize for the surface to the orchestrator through the client

### Requirement: Connection status indicator

The terminal pane SHALL display a visual status indicator reflecting the current attachment state to
the surface stream: connecting, connected, or disconnected. The pane SHALL provide a manual
reconnect control that reattaches to the surface.

#### Scenario: Status updates on connection events

- **WHEN** the attachment to the surface stream transitions between states
- **THEN** the status indicator updates accordingly (e.g. yellow while connecting, green when connected, red when disconnected)

#### Scenario: Manual reconnect

- **WHEN** the user activates the reconnect control
- **THEN** the existing attachment is closed, the terminal buffer is cleared, and a new attachment to the surface opens
