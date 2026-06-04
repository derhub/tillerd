# ui-terminal-pane

## Purpose

Defines the terminal pane that connects to a session over a WebSocket, renders raw agent output, propagates resize events, and surfaces connection status with a manual reconnect control.

## Requirements

### Requirement: Session-scoped terminal connection

The terminal pane SHALL accept a session ID as input and establish a WebSocket connection scoped to that session. When the session ID changes, the previous connection SHALL be torn down and a new one established.

#### Scenario: Connection opens for session

- **WHEN** the terminal pane mounts with a given session ID
- **THEN** it establishes a WebSocket connection carrying that session's identifier and begins streaming output

#### Scenario: Reconnect on session change

- **WHEN** the active session ID changes (e.g. user navigates to a different session)
- **THEN** the prior connection is closed, the terminal buffer is cleared, and a new connection opens for the new session ID

#### Scenario: Connection teardown on unmount

- **WHEN** the terminal pane unmounts
- **THEN** the WebSocket connection is closed cleanly

### Requirement: Terminal output rendering

The terminal pane SHALL render agent output as it arrives, preserving ANSI escape sequences and raw bytes. The terminal SHALL auto-fit to the pane's container dimensions and emit resize events to the server when the container size changes.

#### Scenario: Output streams to display

- **WHEN** the server sends output data for the session
- **THEN** the terminal renders it immediately including colors, cursor movement, and other control sequences

#### Scenario: Resize propagates

- **WHEN** the terminal container is resized (window resize or panel drag)
- **THEN** the pane recalculates dimensions and sends a resize notification to the server

### Requirement: Connection status indicator

The terminal pane SHALL display a visual status indicator reflecting the current connection state: connecting, connected, or disconnected. The pane SHALL provide a manual reconnect control.

#### Scenario: Status updates on connection events

- **WHEN** the WebSocket transitions between states
- **THEN** the status indicator updates accordingly (e.g. yellow while connecting, green when connected, red when disconnected)

#### Scenario: Manual reconnect

- **WHEN** the user activates the reconnect control
- **THEN** the existing connection is closed, the terminal buffer is cleared, and a new connection opens
