# settings-panel Specification

## Purpose

Defines the settings panel: a non-modal panel in the app-shell chrome that lets the user choose the application theme (light / dark, applied from first paint) and the terminal color scheme (applied live to terminal surfaces), persisting both through the settings store.

## Requirements

### Requirement: Settings panel affordance

The system SHALL present a control in the application chrome that opens a non-modal settings panel, and a control that closes it. The panel SHALL render immediately without waiting on background services.

#### Scenario: Open the panel from chrome

- **WHEN** the user activates the settings control in the app chrome
- **THEN** the settings panel becomes visible without blocking the rest of the shell

#### Scenario: Close the panel

- **WHEN** the user activates the close control while the panel is open
- **THEN** the panel is dismissed and the underlying shell remains interactive

### Requirement: Theme selection

The system SHALL let the user choose between a light and a dark appearance, apply the choice immediately, persist it, and restore it on relaunch. The application SHALL render with the persisted appearance from first paint (no flash).

#### Scenario: Switching to light applies and persists

- **WHEN** the user selects the light appearance in the settings panel
- **THEN** the interface switches to light appearance and the choice is persisted to the durable settings store

#### Scenario: Theme restored from first paint

- **WHEN** a non-default appearance was previously chosen and the application is relaunched
- **THEN** the interface renders with the previously chosen appearance from the first paint

### Requirement: Terminal color scheme selection

The system SHALL let the user select a color scheme for terminal surfaces, apply it to existing and new terminal surfaces without restarting them, and persist the choice across relaunch.

#### Scenario: Selecting a scheme applies to terminals

- **WHEN** the user selects a terminal color scheme in the settings panel
- **THEN** terminal surfaces render with the selected scheme and the choice is persisted

#### Scenario: Persisted scheme resolves on load

- **WHEN** a terminal surface is created and a non-default scheme was previously chosen
- **THEN** the surface resolves and applies the persisted scheme
