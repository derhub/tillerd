## MODIFIED Requirements

### Requirement: Recursive panel tree rendering

The shell SHALL render a layout described by a recursive tree. A panel group node SHALL contain two or more children (each a leaf or group) and a `direction` (horizontal/vertical) and `displayMode`. A panel leaf bound to a `placement` SHALL render the surface at that placement; an empty leaf SHALL render a picker. A leaf SHALL NOT own a surface id. The panel tree SHALL hold only session surfaces and empty geometry.

#### Scenario: Nested groups compose

- **WHEN** a panel leaf is replaced by a panel group node
- **THEN** that group's children render within the space previously occupied by the leaf, without affecting any other panel

#### Scenario: Leaf renders its placement's surface

- **WHEN** a panel leaf bound to a placement is rendered
- **THEN** it renders the session surface at that placement, and switching sessions renders the new session's surface at the same placement

### Requirement: Default layout

On first load with no stored geometry, the shell SHALL render the sidebar and host-status badge as chrome outside the panel tree, and a panel tree of one panel per placement in the session's launch spec. A fresh session has an empty launch spec, so the default panel tree SHALL be a single empty leaf; the user spawns the first surface from it. The sidebar and status badge SHALL NOT appear as panels.

#### Scenario: Fresh session renders an empty leaf with chrome

- **WHEN** a fresh session with an empty launch spec is opened and no stored geometry exists
- **THEN** the panel tree is a single empty leaf, with the sidebar and status badge rendered as chrome outside the tree

#### Scenario: Session with surfaces renders one panel per placement

- **WHEN** a session whose spec has placements is opened with no stored geometry
- **THEN** the panel tree renders one panel per placement, in spec order, with the sidebar and status badge as chrome

## REMOVED Requirements

### Requirement: Sidebar display mode

**Reason**: The sidebar is chrome (app-shell UI), not a session surface and not a panel display mode. Per ADR-0030 the panel tree holds only session surfaces and empty geometry; the sidebar renders in the app shell outside the tree.

**Migration**: Render the session list in the app shell as chrome. No panel group uses `displayMode: 'sidebar'`; remove the sidebar panel from any stored layout during reconciliation against the launch spec.
