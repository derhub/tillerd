## MODIFIED Requirements

### Requirement: Interactive chrome carries ARIA semantics

All interactive chrome elements SHALL use semantic controls and expose appropriate roles, accessible names, and state attributes. Every icon-only action SHALL expose an action-specific accessible name and a tooltip on hover or focus. Nested sidebar actions SHALL remain keyboard reachable when revealed by focus, not only by pointer hover. The terminal canvas is exempt from screen-reader support.

#### Scenario: Icon-only button is named

- **WHEN** an icon-only chrome action renders
- **THEN** it exposes an action-specific accessible name and shows the same action description on hover or focus

#### Scenario: Menus expose menu semantics

- **WHEN** a context menu opens
- **THEN** it exposes menu/menuitem roles and the trigger row keeps its accessible state

#### Scenario: Nested sidebar action is keyboard reachable

- **WHEN** keyboard focus enters a project, session, workspace, or archived-item row
- **THEN** every available nested action can receive focus without requiring pointer hover

#### Scenario: Focus is visible at every chrome stop

- **WHEN** keyboard focus reaches an interactive chrome control
- **THEN** the control renders a visible focus indicator using the ring token

### Requirement: Keyboard navigation through chrome

Chrome SHALL be operable by keyboard. Tab and Shift+Tab SHALL traverse standalone controls, arrow keys SHALL move within composite widgets, Enter or Space SHALL activate the focused action, and Escape SHALL dismiss the innermost open overlay and restore focus to its trigger. The terminal canvas is exempt.

#### Scenario: Sidebar tree keyboard traversal

- **WHEN** focus is in the sessions tree
- **THEN** arrow keys move between visible project and session rows, Enter activates the focused row, and nested row actions remain reachable by Tab

#### Scenario: Dialog focus containment

- **WHEN** a confirmation dialog opens
- **THEN** focus moves into the dialog, Tab cycles within it, and Escape closes it returning focus to the trigger

#### Scenario: Menu focus round trip

- **WHEN** a menu opens from a chrome trigger
- **THEN** focus moves into the menu and Escape closes it and restores focus to its trigger

### Requirement: Token pairs meet WCAG AA contrast

Every foreground/background pairing rendered as text at rest or as an interactive state in light and dark themes SHALL meet WCAG AA contrast: 4.5:1 for normal text and 3:1 for large text and user-interface components. A decorative pairing not required to identify or operate a control MAY remain below threshold only when the design system records its measured ratio, rendered use, and rationale. Frozen token values SHALL remain unchanged; failing rendered pairings SHALL use an existing compliant token.

#### Scenario: Muted text on background passes

- **WHEN** muted-foreground text renders on the background color in either theme
- **THEN** the measured contrast ratio meets the AA threshold

#### Scenario: Rendered chrome contrast audit passes

- **WHEN** the application renders representative chrome states in light and dark themes
- **THEN** every required text and user-interface pairing meets its threshold and the design system records the measured results
