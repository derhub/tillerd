## ADDED Requirements

### Requirement: Title bar chrome

On the desktop host the shell SHALL render a fixed-height title bar row spanning the full window width above the shell body. The title bar SHALL provide a draggable region that moves the window when dragged. In the browser build (no Tauri host) the title bar SHALL still render its toolbar controls but SHALL omit the OS window controls and the drag behavior SHALL no-op.

#### Scenario: Dragging the title bar moves the window

- **WHEN** the user presses and drags an empty area of the title bar on the desktop host
- **THEN** the window follows the pointer

#### Scenario: Browser build omits OS controls

- **WHEN** the shell renders without a Tauri host
- **THEN** the title bar renders its toggle toolbar and no OS window controls appear

### Requirement: OS window controls

The title bar SHALL render native-looking OS window controls for minimize, maximize/restore, and close, positioned per the host platform's convention. Activating a control SHALL invoke the corresponding operation on the current window through the desktop window boundary.

#### Scenario: Minimize control minimizes the window

- **WHEN** the user activates the minimize control
- **THEN** the current window is minimized

#### Scenario: Maximize control toggles maximization

- **WHEN** the user activates the maximize control on a non-maximized window
- **THEN** the window is maximized, and activating it again restores the previous size

#### Scenario: Close control closes the window

- **WHEN** the user activates the close control
- **THEN** the current window closes

### Requirement: Panel toggle toolbar

The title bar SHALL host a toolbar with buttons that toggle the visibility of the left sidebar, the right dock, and the bottom dock, plus a button that toggles the command palette. Each panel toggle button SHALL reflect the current visibility of its target region.

#### Scenario: Toggling a panel hides and shows it

- **WHEN** the user activates the left-sidebar toggle while the sidebar is visible
- **THEN** the sidebar is hidden, and activating the toggle again shows it

#### Scenario: Toggle button reflects region state

- **WHEN** the right dock is hidden
- **THEN** the right-dock toggle button renders in its inactive/off state

#### Scenario: Command toggle opens the palette

- **WHEN** the user activates the command toggle while the command palette is closed
- **THEN** the command palette opens

### Requirement: Left, right, and bottom dock regions

The shell body SHALL host the existing left sidebar plus a right dock and a bottom dock region flanking the content outlet. Each region SHALL be independently hideable. When a region is hidden it SHALL not occupy layout space, and the content outlet SHALL reclaim that space. The right and bottom docks MAY render placeholder content until their content is defined.

#### Scenario: Hidden region reclaims space

- **WHEN** the bottom dock is hidden
- **THEN** the bottom dock occupies no vertical space and the content area extends to the window bottom

#### Scenario: Regions are independently controlled

- **WHEN** the user hides the right dock
- **THEN** the left sidebar and bottom dock visibility are unchanged

### Requirement: Persisted panel visibility

The visibility of the left sidebar, right dock, and bottom dock SHALL persist across application restarts via the settings store, following the existing durable-settings pattern. On launch each region SHALL restore its last persisted visibility, defaulting to a defined initial state when no value is stored.

#### Scenario: Visibility survives restart

- **WHEN** the user hides the right dock and restarts the application
- **THEN** the right dock is hidden on next launch

#### Scenario: Default visibility on first launch

- **WHEN** the application launches with no stored visibility for a region
- **THEN** that region renders in its defined default visibility state

### Requirement: Toggles are command-center actions

Each panel toggle and the command toggle SHALL be registered as command-center actions with stable action ids, so they are invocable from the command palette and rebindable through the keybinding system. Invoking an action from the palette SHALL have the same effect as activating its title bar button.

#### Scenario: Toggle invoked from the palette

- **WHEN** the user invokes the "toggle right dock" action from the command palette
- **THEN** the right dock visibility toggles, identically to the title bar button
