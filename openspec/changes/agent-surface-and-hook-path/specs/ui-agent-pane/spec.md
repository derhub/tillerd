## ADDED Requirements

### Requirement: Agent surface renders a terminal pane embedded in an agent pane

The UI SHALL render an agent surface as an `AgentPane` component that embeds the full raw-byte terminal display. The terminal display within `AgentPane` SHALL behave identically to `TerminalPane` for PTY byte streaming, input, and resize.

#### Scenario: Agent surface dispatches to AgentPane

- **WHEN** the panel renderer encounters a surface with `kind = agent`
- **THEN** it SHALL render `AgentPane` instead of `TerminalPane`

#### Scenario: Terminal output streams inside AgentPane

- **WHEN** the agent process writes bytes to the PTY
- **THEN** the embedded terminal display inside `AgentPane` SHALL render those bytes unchanged

### Requirement: Status badge displays current agent lifecycle state

`AgentPane` SHALL display a status badge that reflects the agent's current lifecycle state. The badge SHALL show one of five states: `IDLE`, `WORKING`, `WAITING_INPUT`, `DONE`, or `crashed`. The badge SHALL update each time a status event is received for that surface.

#### Scenario: Badge shows IDLE on initial open

- **WHEN** an agent surface is first opened and no hook event has been received yet
- **THEN** the status badge SHALL show `IDLE`

#### Scenario: Badge updates to WORKING on working status

- **WHEN** a `WORKING` status event is received for the surface
- **THEN** the status badge SHALL update to `WORKING`

#### Scenario: Badge updates to WAITING_INPUT on permission request

- **WHEN** a `WAITING_INPUT` status event is received for the surface
- **THEN** the status badge SHALL update to `WAITING_INPUT`

#### Scenario: Badge updates to DONE on session end

- **WHEN** a `DONE` status event is received for the surface
- **THEN** the status badge SHALL update to `DONE`

#### Scenario: Badge shows crashed on non-zero exit

- **WHEN** the surface exits with a non-clean qualifier
- **THEN** the status badge SHALL show `crashed`

### Requirement: Content stream displays tool-use events

`AgentPane` SHALL display a scrollable content area that lists `ContentEvent` entries received for the surface. Each `tool_use` entry SHALL show the tool name and tool input. New entries SHALL be appended without clearing prior entries.

#### Scenario: Tool-use entry appended on content event

- **WHEN** a `tool_use` content event is received
- **THEN** `AgentPane` SHALL append a new entry showing the tool name and tool input

#### Scenario: Earlier entries remain visible

- **WHEN** a second content event arrives
- **THEN** the first content entry SHALL still be visible in the content area

### Requirement: Failure state rendered for gate subscription error

`AgentPane` SHALL render a typed failure state when a gate subscription error is received for the surface. The failure state SHALL be distinct from normal terminal output and SHALL not prevent the terminal display from showing PTY bytes already received.

#### Scenario: Error overlay shown on subscription failure

- **WHEN** the surface runtime emits a gate error for the surface
- **THEN** `AgentPane` SHALL display a failure indicator describing the error

#### Scenario: Prior PTY bytes remain visible during failure

- **WHEN** a gate error is displayed
- **THEN** the terminal area SHALL still show bytes received before the error occurred
