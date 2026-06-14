## ADDED Requirements

### Requirement: Host creates and focuses child windows on request

The host SHALL expose IPC commands for the renderer to open a child window at a renderer route
(by label) and to raise an existing window to the front by its label. All windows SHALL share the
single embedded orchestrator backend; opening a child window SHALL NOT spawn a backend. A window
closes itself through the core window API; closing one SHALL NOT tear down the backend.

#### Scenario: Opening a child window

- **WHEN** the renderer invokes the create-window command with a label and a renderer route
- **THEN** a child window opens loading that route against the same backend

#### Scenario: Focusing a window by label

- **WHEN** the renderer invokes the focus-window command with an existing window label
- **THEN** that window is raised to the front

#### Scenario: A child window closes without affecting the backend

- **WHEN** a child window closes itself
- **THEN** only that window closes and the shared backend keeps running

## MODIFIED Requirements

### Requirement: Application lifecycle drives a graceful shutdown

The system SHALL initiate a graceful shutdown of the background processes it owns when the
application exits — that is, when its last open window closes. Closing a non-last window SHALL
close only that window and SHALL NOT trigger the shutdown sequence.

#### Scenario: Closing the last window

- **WHEN** the user closes the last open application window
- **THEN** the application signals its owned background processes to shut down gracefully
- **AND** the application terminates only after that shutdown sequence has been initiated

#### Scenario: Closing a parent window while a child remains open

- **WHEN** the user closes a window while another application window is still open
- **THEN** only that window closes
- **AND** the owned background processes keep running
