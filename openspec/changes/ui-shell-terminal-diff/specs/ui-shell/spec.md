## ADDED Requirements

### Requirement: Recursive panel tree rendering

The shell SHALL render a layout described by a recursive tree. A panel group node SHALL contain two or more children (each a leaf or group) and a `direction` (horizontal/vertical) and `displayMode`. A panel leaf SHALL render one content component.

#### Scenario: Nested groups compose

- **WHEN** a panel leaf is replaced by a panel group node
- **THEN** that group's children render within the space previously occupied by the leaf, without affecting any other panel

### Requirement: Split display mode

When a panel group has `displayMode: 'split'`, all child panels SHALL be visible simultaneously with independent resize handles between siblings. Each panel SHALL enforce a minimum size (sidebar: 180px, terminal: 300px, diff: 240px). Panel sizes SHALL persist per group.

#### Scenario: Resize handle adjusts siblings only

- **WHEN** the user drags a handle between panels A and B in a group A, B, C
- **THEN** only A and B change size; C is unaffected

#### Scenario: Minimum size enforced

- **WHEN** a drag would reduce a panel below its minimum
- **THEN** the drag stops at the minimum

### Requirement: Tabbar-top display mode

When a panel group has `displayMode: 'tabbar-top'`, a tab bar SHALL appear above the content area. Each tab SHALL show the panel's title. Only the active tab's panel SHALL be rendered. Clicking a tab SHALL make that panel active.

#### Scenario: Tabs render panel titles

- **WHEN** a group is in tabbar-top mode with three panels
- **THEN** three tabs appear above the content area, each labeled with the corresponding panel's title

#### Scenario: Only active panel rendered

- **WHEN** the user clicks tab B in a group with tabs A, B, C
- **THEN** panel B's content is rendered; panels A and C are not mounted

### Requirement: Tabbar-bottom display mode

When a panel group has `displayMode: 'tabbar-bottom'`, the tab bar SHALL appear below the content area. Behavior is otherwise identical to `tabbar-top`.

#### Scenario: Tab bar positioned below

- **WHEN** a group has displayMode tabbar-bottom
- **THEN** the tab strip renders beneath the panel content area

### Requirement: Sidebar display mode

When a panel group has `displayMode: 'sidebar'`, the group SHALL render a vertical list of panel titles on the left side. Only the active panel's content SHALL be expanded. Clicking a title item SHALL expand that panel's content and collapse the previously active one.

#### Scenario: Sidebar lists panel titles

- **WHEN** a group is in sidebar mode with four panels
- **THEN** four title items appear in a vertical list

#### Scenario: Click expands panel

- **WHEN** the user clicks a sidebar item
- **THEN** that panel's content expands; the previously active panel collapses

### Requirement: Panel split action

Each panel SHALL provide split-horizontal and split-vertical actions in its toolbar. Activating either SHALL replace the panel leaf with a group node (direction: horizontal or vertical, mode: split) containing the original panel and a new empty panel.

#### Scenario: Horizontal split

- **WHEN** the user activates "split right" on a terminal panel
- **THEN** the panel is replaced by a horizontal split group containing the original terminal and a new empty panel

#### Scenario: Vertical split

- **WHEN** the user activates "split down" on any panel
- **THEN** the panel is replaced by a vertical split group with the original panel and a new empty panel

### Requirement: Default layout

On first load with no stored layout, the shell SHALL initialize with a horizontal split group containing a sidebar panel (title: "Sessions"), a terminal panel (title: "Terminal"), and a diff panel (title: "Changes"), in that order.

#### Scenario: Fresh load

- **WHEN** no stored layout exists
- **THEN** the three-column default renders

### Requirement: Compact visual style

Panel headers SHALL be 24px tall. Toolbar chrome SHALL be 28px tall. Base font SHALL be 12px. Borders SHALL be 1px. Panel container border radius SHALL be 0. Resize handles SHALL be invisible at rest and 1px highlighted on hover.

#### Scenario: Panel header height

- **WHEN** any panel is rendered
- **THEN** its header strip is 24px tall

#### Scenario: Resize handle on hover

- **WHEN** the user hovers over a resize handle
- **THEN** it becomes visible as a 1px highlighted line
