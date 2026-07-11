# ui-settings-editor Specification

## Purpose
TBD - created by archiving change ux-ui-overhaul. Update Purpose after archive.
## Requirements
### Requirement: Settings editor surface

The application SHALL present a full settings editor at its own route, rendered in the
panel area (not a session surface, consuming no placement). The editor SHALL present
navigable sections: General, Appearance, Terminal, Keybindings, Profiles, Themes, and —
when a project context is active — Project. It SHALL
render immediately without waiting on background services and SHALL be reachable from the
status bar, the command palette, and the activity bar's settings affordance.

#### Scenario: Opening the editor

- **WHEN** the user activates a settings affordance
- **THEN** the settings editor renders in the panel area with its section navigation, and
  the active session's panel tree is restored when the user navigates back

#### Scenario: Section navigation

- **WHEN** the user selects the Keybindings section
- **THEN** the keybinding settings render without a full page reload

### Requirement: Appearance settings

The Appearance section SHALL let the user choose between light and dark appearance, apply
the choice immediately, persist it, and restore it from first paint on relaunch (no
flash).

#### Scenario: Switching to light applies and persists

- **WHEN** the user selects the light appearance
- **THEN** the interface switches to light appearance and the choice is persisted to the
  durable settings store

#### Scenario: Theme restored from first paint

- **WHEN** a non-default appearance was previously chosen and the application is
  relaunched
- **THEN** the interface renders with the previously chosen appearance from the first
  paint

### Requirement: Terminal settings

The Terminal section SHALL let the user set the terminal color scheme, font size, font
family, line height, cursor style, cursor blink, scrollback size, copy-on-select, and
confirm-before-paste — each applied to existing and new terminal surfaces without
restarting them and persisted across relaunch.

#### Scenario: Selecting a scheme applies to terminals

- **WHEN** the user selects a terminal color scheme
- **THEN** terminal surfaces render with the selected scheme and the choice is persisted

#### Scenario: Persisted scheme resolves on load

- **WHEN** a terminal surface is created and a non-default scheme was previously chosen
- **THEN** the surface resolves and applies the persisted scheme

### Requirement: General settings

The General section SHALL offer: the close-surface confirmation toggle (mirrors the
dialog's don't-ask-again preference), the startup workspace (last-used or a pinned
choice), and the UI zoom level (applied live, persisted).

#### Scenario: Zoom applies live

- **WHEN** the user changes the UI zoom level
- **THEN** the window content scales immediately and the level persists across relaunch

### Requirement: Project settings

When a project context is active, the Project section SHALL expose project-scoped
settings, at minimum the project's default launch template (used by the new-session
flow); values persist in the project scope of the settings store.

#### Scenario: Default template honored

- **WHEN** a project's default template is set and the user creates a session with the
  plain new-session control
- **THEN** the session instantiates that template

### Requirement: Keybinding settings

The Keybindings section SHALL present the preset selector and the per-command binding
list with override editing and per-command and global reset, replacing the popover-hosted
keybinding settings. Existing keybinding behavior (presets, overrides, persistence) is
unchanged.

#### Scenario: Rebinding from the editor

- **WHEN** the user assigns a new key to a command in the Keybindings section
- **THEN** the command's resolved binding becomes that key and persists across restart

### Requirement: Profile management

The Profiles section SHALL list settings profiles with the active one indicated and SHALL
support creating, activating, renaming, duplicating, deleting, exporting, and importing a
profile. Activating a profile SHALL apply its settings without restart. Deleting the
active profile SHALL be guarded by a confirmation.

#### Scenario: Switching the active profile

- **WHEN** the user activates another profile
- **THEN** that profile becomes active and its settings take effect

#### Scenario: Duplicating a profile

- **WHEN** the user duplicates a profile under a new name
- **THEN** a new profile with identical settings appears in the list

### Requirement: Theme management

The Themes section SHALL list available themes with the active one indicated and SHALL
support activating, importing, exporting, and deleting a theme. Prebuilt themes SHALL NOT
be deletable.

#### Scenario: Activating a theme

- **WHEN** the user activates a theme from the list
- **THEN** the theme is applied and persisted

#### Scenario: Prebuilt theme delete is rejected

- **WHEN** the user attempts to delete a prebuilt theme
- **THEN** no delete affordance is offered (or the action is disabled) for that theme

