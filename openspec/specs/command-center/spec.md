# command-center Specification

## Purpose

The command center is a leader-key–activated, fuzzy-searchable palette over the application's chrome
actions, backed by a configurable keybinding layer (built-in preset baselines plus per-action
overrides). The leader key is registered at the host's native menu level so it fires regardless of
which surface holds keyboard focus, including an active terminal.

## Requirements

### Requirement: Leader key opens the command center

A configurable leader key SHALL open the command center overlay. On the desktop host it MUST be
registered at the native menu level so it fires regardless of which surface holds keyboard focus,
including an active terminal surface.

#### Scenario: Leader key opens the palette

- **WHEN** the user presses the configured leader key
- **THEN** the command center overlay opens with the search field focused

#### Scenario: Leader key fires while a terminal holds focus

- **WHEN** a terminal surface has keyboard focus and the user presses the leader key
- **THEN** the overlay opens and the keystroke is not delivered to the terminal

#### Scenario: Changing the leader key takes effect

- **WHEN** the leader key is reconfigured to a different key
- **THEN** the new key opens the overlay and the previous key no longer does

### Requirement: Command palette lists and invokes actions

The overlay SHALL present the available actions in a fuzzy-searchable list. Selecting an action
MUST invoke the same handler as that action's existing control and then close the overlay.

#### Scenario: Overlay lists available actions

- **WHEN** the overlay opens
- **THEN** it lists every action currently available in context

#### Scenario: Query filters the list

- **WHEN** the user types a query
- **THEN** the list narrows to fuzzy matches ordered best-match first

#### Scenario: Selecting an action invokes its handler

- **WHEN** the user selects an action
- **THEN** that action's handler runs and the overlay closes

#### Scenario: Dismissing closes without invoking

- **WHEN** the user dismisses the overlay with the cancel key or by clicking outside it
- **THEN** the overlay closes and no action is invoked

#### Scenario: Action shows its resolved binding

- **WHEN** the overlay lists an action that has a resolved key binding
- **THEN** the binding is displayed beside that action

### Requirement: Bindings are configurable and persist

Every action SHALL have a rebindable key. The selected preset and any per-action overrides MUST
persist across restarts. An action's configured binding MUST invoke the action from the renderer
when no terminal surface holds keyboard focus.

#### Scenario: Overriding a binding changes the resolved key

- **WHEN** the user assigns a new key to an action
- **THEN** the action's resolved binding becomes that key

#### Scenario: Overrides survive restart

- **WHEN** an override is set and the application is restarted
- **THEN** the override is still in effect

#### Scenario: Configured key invokes the action when the terminal is unfocused

- **WHEN** no terminal surface holds keyboard focus and the user presses an action's configured key
- **THEN** that action's handler runs

#### Scenario: Clearing an override falls back to the preset

- **WHEN** the user clears an action's override
- **THEN** the action resolves to its current preset's binding

### Requirement: Preset profiles provide binding baselines

The system SHALL ship built-in keybinding presets. Selecting a preset MUST set the baseline
binding for every action; per-action overrides layer on top of the active preset. A default preset
MUST be active on first run.

#### Scenario: Default preset is active on first run

- **WHEN** the application starts with no stored keybinding configuration
- **THEN** the default preset's bindings are in effect

#### Scenario: Selecting a preset applies its baseline

- **WHEN** the user selects a preset
- **THEN** every action without an override resolves to that preset's binding

#### Scenario: Override wins over the preset

- **WHEN** an action has an override and a preset is active
- **THEN** the action resolves to the override and all other actions resolve to the preset

#### Scenario: Switching presets preserves overrides

- **WHEN** the user switches to a different preset
- **THEN** unoverridden actions follow the new preset and existing overrides remain in effect
