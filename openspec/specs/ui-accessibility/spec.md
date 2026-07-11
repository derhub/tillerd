# ui-accessibility Specification

## Purpose
TBD - created by archiving change ux-ui-overhaul. Update Purpose after archive.
## Requirements
### Requirement: Interactive chrome carries ARIA semantics

All interactive chrome elements (activity bar, sidebar rows and view headers, panel header toolbars, status bar items, dialogs, menus, tabs, tooltips) SHALL carry appropriate ARIA roles, accessible names, and state attributes. Icon-only buttons SHALL have an accessible name and a tooltip. The terminal canvas is exempt from screen-reader support.

#### Scenario: Icon-only button is named

- **WHEN** a panel header split button renders
- **THEN** it exposes an accessible name and shows a tooltip on hover/focus

#### Scenario: Menus expose menu semantics

- **WHEN** a context menu opens
- **THEN** it exposes menu/menuitem roles and the trigger row keeps its accessible state

### Requirement: Keyboard navigation through chrome

Chrome SHALL be operable by keyboard: Tab/Shift+Tab traverse focusable chrome regions,
arrow keys move within composite widgets (menus, tab strips, the sidebar tree), Enter
activates, and Escape dismisses the innermost open overlay. Focus SHALL be visible via
the ring token at every stop.

#### Scenario: Sidebar tree keyboard traversal

- **WHEN** focus is in the sessions tree
- **THEN** arrow keys move between project and session rows, Enter opens the focused
  session, and Escape returns focus to the tree from an opened menu

#### Scenario: Dialog focus containment

- **WHEN** a confirmation dialog opens
- **THEN** focus moves into the dialog, Tab cycles within it, and Escape closes it
  returning focus to the trigger

### Requirement: Token pairs meet WCAG AA contrast

Every foreground/background token pairing rendered as text at rest or as an interactive state, in both light and dark mode, SHALL meet WCAG AA contrast (4.5:1 normal text, 3:1 large text and UI components). The verified pairings SHALL be recorded in DESIGN.md.
A pairing never rendered at rest, or a deliberate sub-threshold hairline whose
interactive states pass independently, MAY fall below the threshold when DESIGN.md
records the measured ratio and the rationale.

#### Scenario: Muted text on background passes

- **WHEN** muted-foreground text renders on the background color in either theme
- **THEN** the measured contrast ratio meets the AA threshold

### Requirement: Light mode renders every component correctly

Every chrome component SHALL render correctly in light mode using the light token
counterparts, with no hardcoded dark-mode colors outside the terminal palette. The
terminal canvas stays dark in both themes by design.

#### Scenario: Light-mode sweep

- **WHEN** the application runs in light mode
- **THEN** workbench regions, menus, dialogs, manager views, and overlays all render with
  light tokens while the terminal canvas remains dark

