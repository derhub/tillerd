# desktop-title-bar

## Purpose

The desktop shell's title bar row: native window controls, a draggable region, a panel-toggle
toolbar, and the resizable left/right/bottom dock regions it controls.
## Requirements
### Requirement: Title bar chrome

On the desktop host the shell SHALL render a fixed-height title bar row spanning the full window width above the shell body. The title bar SHALL provide a draggable region that moves the window when dragged. In the browser build (no Tauri host) the title bar SHALL still render its toolbar controls but SHALL omit the OS window controls and the drag behavior SHALL no-op.

#### Scenario: Dragging the title bar moves the window

- **WHEN** the user presses and drags an empty area of the title bar on the desktop host
- **THEN** the window follows the pointer

#### Scenario: Browser build omits OS controls

- **WHEN** the shell renders without a Tauri host
- **THEN** the title bar renders its toggle toolbar and no OS window controls appear

### Requirement: OS window controls

The desktop window SHALL present the operating system's native window controls (minimize, maximize/restore, close) in their platform-default position. The window SHALL be configured with an overlay title bar so the native controls render over the custom title bar; the shell SHALL NOT draw its own minimize/maximize/close buttons. The title bar SHALL reserve space for the native controls so the toolbar sits inline beside them.

#### Scenario: Native controls are available on the desktop host

- **WHEN** the desktop window is shown
- **THEN** the OS-native minimize, maximize/restore, and close controls are present in their platform-default position

#### Scenario: The title bar draws no control buttons

- **WHEN** the title bar renders
- **THEN** it renders no minimize/maximize/close buttons of its own; the operating system provides them

### Requirement: Panel toggle toolbar

The title bar SHALL host a toolbar with buttons that toggle the visibility of the primary
sidebar and the bottom panel, plus a button that toggles the command palette. Each panel
toggle button SHALL reflect the current visibility of its target region.

#### Scenario: Toggling a panel hides and shows it

- **WHEN** the user activates the sidebar toggle while the sidebar is visible
- **THEN** the sidebar is hidden, and activating the toggle again shows it

#### Scenario: Toggle button reflects region state

- **WHEN** the bottom panel is hidden
- **THEN** the bottom-panel toggle button renders in its inactive/off state

#### Scenario: Command toggle opens the palette

- **WHEN** the user activates the command toggle while the command palette is closed
- **THEN** the command palette opens

### Requirement: Toggles are command-center actions

Each panel toggle and the command toggle SHALL be registered as command-center actions with stable action ids, so they are invocable from the command palette and rebindable through the keybinding system. Invoking an action from the palette SHALL have the same effect as activating its title bar button.

#### Scenario: Toggle invoked from the palette

- **WHEN** the user invokes the "toggle right dock" action from the command palette
- **THEN** the right dock visibility toggles, identically to the title bar button

