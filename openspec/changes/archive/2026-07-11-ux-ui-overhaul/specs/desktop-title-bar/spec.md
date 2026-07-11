# desktop-title-bar

## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: Left, right, and bottom dock regions

**Reason**: Superseded by the `ui-workbench` region model (activity bar, primary sidebar,
bottom panel, status bar); the right dock is removed.
**Migration**: Region layout, hide/resize, and space-reclaim behavior are specified by
`ui-workbench` "Workbench regions".

### Requirement: Persisted panel visibility

**Reason**: Superseded by workbench-wide state persistence covering visibility, sizes,
active view, and active tab.
**Migration**: `ui-workbench` "Workbench state persists".
