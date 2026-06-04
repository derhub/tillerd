## ADDED Requirements

### Requirement: Native window hosts the renderer

The system SHALL provide a native desktop application that hosts the existing web renderer in
an embedded system web view, presenting the renderer as a native window without requiring a
separate web browser.

#### Scenario: Launching the desktop application

- **WHEN** the user launches the desktop application
- **THEN** a native window opens displaying the renderer
- **AND** no external web browser is required

#### Scenario: Renderer reuse

- **WHEN** the desktop application loads its renderer
- **THEN** it loads the same renderer used by the web deployment, with no behavioral fork in
  the user-facing interface

### Requirement: Application lifecycle drives a graceful shutdown

The system SHALL, on application exit, initiate a graceful shutdown of the background processes
it owns before the application terminates.

#### Scenario: Closing the application window

- **WHEN** the user closes the application window
- **THEN** the application signals its owned background processes to shut down gracefully
- **AND** the application terminates only after that shutdown sequence has been initiated
