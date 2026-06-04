# ui-panel-model

## Purpose

Defines the panel tree state model: leaf titles, group display modes, toolbar configuration, persistence to local storage, and content-type assignment for empty panels.

## Requirements

### Requirement: Panel leaf title

Every panel leaf node SHALL have a required non-empty title string. The title SHALL be displayed in the panel's header, in any tab bar that contains the panel, and in any sidebar accordion that contains the panel.

#### Scenario: Title appears in header

- **WHEN** a panel leaf is rendered
- **THEN** its title appears in the panel header

#### Scenario: Title appears in tab

- **WHEN** a panel group is in tabbar mode and the panel is a child of that group
- **THEN** the panel's title appears as the label of its tab

#### Scenario: Title appears in sidebar item

- **WHEN** a panel group is in sidebar mode and the panel is a child of that group
- **THEN** the panel's title appears as the label of its sidebar item

### Requirement: Panel group display mode

Every panel group node SHALL carry a `displayMode` field with one of four values: `split`, `tabbar-top`, `tabbar-bottom`, or `sidebar`. The default for new groups created by a split action SHALL be `split`. Display mode SHALL be persisted as part of the layout tree in local storage.

#### Scenario: Default mode on split

- **WHEN** the user splits a panel
- **THEN** the resulting group node has display mode `split`

#### Scenario: Display mode persists

- **WHEN** a panel group has display mode set and the page is reloaded
- **THEN** the group renders in the same display mode as before

### Requirement: Panel toolbar configuration

A panel leaf node MAY carry an optional toolbar configuration consisting of an ordered list of button configurations. Each button configuration SHALL include an icon identifier and a label string. Toolbar configuration SHALL be persisted as part of the layout tree.

#### Scenario: Panel without toolbar

- **WHEN** a panel leaf has no toolbar configuration
- **THEN** no toolbar is rendered in the panel header

#### Scenario: Panel with toolbar buttons

- **WHEN** a panel leaf has toolbar configuration with two buttons
- **THEN** those two buttons are rendered in the panel's header toolbar area

### Requirement: Panel tree state model

The application SHALL maintain a panel tree in React state. The tree SHALL be initialized from `localStorage` on load; on parse failure or missing storage, it SHALL fall back to the default layout. Every structural change (split, content assignment) SHALL be written to `localStorage` immediately.

#### Scenario: Corrupt storage is discarded

- **WHEN** the stored layout is not a valid panel tree
- **THEN** the default layout is used and the invalid storage entry is overwritten

#### Scenario: Split persists

- **WHEN** the user splits a panel and reloads
- **THEN** the split group is restored from storage

### Requirement: Panel content type assignment

An empty panel leaf SHALL present a picker for available content types. Selecting a type SHALL update the leaf's content field and persist the change.

#### Scenario: Empty panel picker

- **WHEN** a panel has content type `empty`
- **THEN** it renders a picker listing available content types

#### Scenario: Assignment persists

- **WHEN** the user assigns a content type to an empty panel
- **THEN** the assignment is written to local storage and survives a reload
