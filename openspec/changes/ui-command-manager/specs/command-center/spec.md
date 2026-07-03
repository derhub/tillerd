## MODIFIED Requirements

### Requirement: Command palette lists and invokes actions

The overlay SHALL present the actions that are tagged for the palette surface and whose context (`when`) currently passes, in a fuzzy-searchable list grouped by category. Selecting an action MUST invoke the same handler as that action's other surfaces (e.g. its title bar control) and then close the overlay. For a toggle action the overlay SHALL display its current checked state.

#### Scenario: Overlay lists palette actions available in context

- **WHEN** the overlay opens
- **THEN** it lists every action tagged for the palette whose `when` currently passes, and omits actions whose `when` is false or that are not tagged for the palette

#### Scenario: Query filters the list

- **WHEN** the user types a query
- **THEN** the list narrows to fuzzy matches ordered best-match first

#### Scenario: Selecting an action invokes its handler

- **WHEN** the user selects an action
- **THEN** that action's registered handler runs and the overlay closes

#### Scenario: Toggle action shows its checked state

- **WHEN** the overlay lists a toggle action whose underlying state is on
- **THEN** the action renders with a checked indicator, and selecting it flips the underlying state

#### Scenario: Dismissing closes without invoking

- **WHEN** the user dismisses the overlay with the cancel key or by clicking outside it
- **THEN** the overlay closes and no action is invoked

#### Scenario: Action shows its resolved binding

- **WHEN** the overlay lists an action that has a resolved key binding
- **THEN** the binding is displayed beside that action
