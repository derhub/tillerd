# ui-shell

## Purpose

Defines the recursive panel-tree shell: split and tabbar display modes, the split action, panes bound to surfaces by placement with the sidebar and status badge as chrome, the empty-leaf default layout, and the compact visual style.

## Requirements

### Requirement: Recursive panel tree rendering

The shell SHALL render a layout described by a recursive tree. A panel group node SHALL contain two or more children (each a leaf or group) and a `direction` (horizontal/vertical) and `displayMode`. A panel leaf bound to a `placement` SHALL render the surface at that placement; an empty leaf SHALL render a picker. A leaf SHALL NOT own a surface id. The panel tree SHALL hold only session surfaces and empty geometry.

#### Scenario: Nested groups compose

- **WHEN** a panel leaf is replaced by a panel group node
- **THEN** that group's children render within the space previously occupied by the leaf, without affecting any other panel

#### Scenario: Leaf renders its placement's surface

- **WHEN** a panel leaf bound to a placement is rendered
- **THEN** it renders the session surface at that placement, and switching sessions renders the new session's surface at the same placement

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

### Requirement: Panel split action

Each panel SHALL provide split-horizontal and split-vertical actions in its toolbar. Activating either SHALL replace the panel leaf with a group node (direction: horizontal or vertical, mode: split) containing the original panel and a new empty panel.

#### Scenario: Horizontal split

- **WHEN** the user activates "split right" on a terminal panel
- **THEN** the panel is replaced by a horizontal split group containing the original terminal and a new empty panel

#### Scenario: Vertical split

- **WHEN** the user activates "split down" on any panel
- **THEN** the panel is replaced by a vertical split group with the original panel and a new empty panel

### Requirement: Default layout

On first load with no stored geometry, the shell SHALL render the sidebar and host-status badge as chrome outside the panel tree, and a panel tree of one panel per placement in the session's launch spec. A fresh session has an empty launch spec, so the default panel tree SHALL be a single empty leaf; the user spawns the first surface from it. The sidebar and status badge SHALL NOT appear as panels.

#### Scenario: Fresh session renders an empty leaf with chrome

- **WHEN** a fresh session with an empty launch spec is opened and no stored geometry exists
- **THEN** the panel tree is a single empty leaf, with the sidebar and status badge rendered as chrome outside the tree

#### Scenario: Session with surfaces renders one panel per placement

- **WHEN** a session whose spec has placements is opened with no stored geometry
- **THEN** the panel tree renders one panel per placement, in spec order, with the sidebar and status badge as chrome

### Requirement: Compact visual style

Panel headers SHALL be 24px tall. Toolbar chrome SHALL be 28px tall. Base font SHALL be 12px. Borders SHALL be 1px. Panel container border radius SHALL be 0. Resize handles SHALL be invisible at rest and 1px highlighted on hover.

#### Scenario: Panel header height

- **WHEN** any panel is rendered
- **THEN** its header strip is 24px tall

#### Scenario: Resize handle on hover

- **WHEN** the user hovers over a resize handle
- **THEN** it becomes visible as a 1px highlighted line
