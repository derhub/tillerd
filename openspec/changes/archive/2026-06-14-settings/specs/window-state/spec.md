## ADDED Requirements

### Requirement: Window geometry persistence

The system SHALL persist the main application window's size, position, and maximized state when they change, and restore them on the next launch. Window geometry SHALL be stored by the host as window state, independently of user settings.

#### Scenario: Size and position restored on relaunch

- **WHEN** the user resizes and moves the window and then relaunches the application
- **THEN** the window reopens at the previously persisted size and position

#### Scenario: Maximized state restored on relaunch

- **WHEN** the window is maximized and the application is relaunched
- **THEN** the window reopens maximized

#### Scenario: First launch uses default geometry

- **WHEN** the application launches with no previously persisted window state
- **THEN** the window opens at the configured default geometry without error
