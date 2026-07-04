# ui-terminal-pane

## ADDED Requirements

### Requirement: Default terminal typography and color scheme

The terminal SHALL render with a bundled default monospace font and the user-selected
color scheme from settings, applied live to existing and new terminals. The scheme maps
onto the terminal token slots; the terminal canvas stays dark in both host themes.

#### Scenario: Bundled font renders offline

- **WHEN** a terminal surface renders with no network access
- **THEN** the bundled monospace font is used (no system-font fallback flash)

#### Scenario: Scheme change applies live

- **WHEN** the user selects a different terminal color scheme in settings
- **THEN** running terminals re-render with the new scheme without restart

### Requirement: Copy and paste

Terminal selection copy and clipboard paste SHALL work with the platform's standard
shortcuts on macOS and Linux, with no Tauri webview conflicts.

#### Scenario: Copy from selection

- **WHEN** the user selects terminal output and presses the platform copy shortcut
- **THEN** the selected text is on the system clipboard

#### Scenario: Paste into the terminal

- **WHEN** the user presses the platform paste shortcut with text on the clipboard
- **THEN** the text is written to the PTY input

### Requirement: Surface failure overlay

A surface-level failure (spawn error, abnormal exit) SHALL render an overlay inside the
pane using the terminal token set, showing the failure reason with resume/dismiss
actions, visually distinct from the service-health indicator.

#### Scenario: Abnormal exit shows the overlay

- **WHEN** a terminal's process exits abnormally
- **THEN** an overlay styled with terminal tokens presents the exit reason and a resume
  action
