## ADDED Requirements

### Requirement: Controlled split geometry

`PanelGroup.Split` SHALL render each child from the owning split group's stored size and SHALL report normalized child sizes after a completed divider resize. Each nested `PanelGroup.Split` SHALL use only its own group's geometry.

#### Scenario: Stored sizes control rendering

- **WHEN** a split group renders with stored sizes `[30, 70]`
- **THEN** its two resizable children use 30 and 70 as their initial sizes

#### Scenario: Divider resize reports normalized sizes

- **WHEN** the user completes a divider drag
- **THEN** the split group reports one normalized size per child for persistence

## MODIFIED Requirements

### Requirement: Divider reset

Double-clicking a resize divider between panels SHALL reset the adjacent panels to an equal split and SHALL report the equal sizes for persistence.

#### Scenario: Double-click resets

- **WHEN** two panels are unevenly sized and the user double-clicks their divider
- **THEN** both panels return to equal size and the owning split group persists `[50, 50]`
