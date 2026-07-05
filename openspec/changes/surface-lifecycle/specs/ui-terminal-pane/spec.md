## MODIFIED Requirements

### Requirement: Connection status indicator

The terminal pane SHALL display a visual status indicator reflecting the current attachment state to
the surface stream: connecting, connected, disconnected, or exited. The pane SHALL provide a manual
reconnect control that reattaches to the surface.

When the surface's process has exited cleanly, the pane SHALL keep its final scrollback visible and
present an inline exit affordance that reports the exit code and offers a Restart control and a New
surface control. Restart SHALL spawn a fresh terminal surface into the pane and rebind the pane's data
channel to it. New surface SHALL request the leaf be reset to an empty picker. The pane SHALL NOT auto-clear or auto-remove on a clean exit; it changes only when the
user acts on the exit affordance or closes the pane.

#### Scenario: Status updates on connection events

- **WHEN** the attachment to the surface stream transitions between states
- **THEN** the status indicator updates accordingly (e.g. yellow while connecting, green when connected, red when disconnected)

#### Scenario: Manual reconnect

- **WHEN** the user activates the reconnect control
- **THEN** the existing attachment is closed, the terminal buffer is cleared, and a new attachment to the surface opens

#### Scenario: Clean exit shows the exit affordance

- **WHEN** the surface's process exits cleanly
- **THEN** the pane keeps its final output visible and shows an inline exit affordance reporting the exit code with Restart and New surface controls

#### Scenario: Restart rebinds the pane to a fresh surface

- **WHEN** the user activates Restart on an exited pane
- **THEN** a fresh terminal surface is spawned into the pane and the pane rebinds its data channel to it
