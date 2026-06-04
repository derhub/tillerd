# ui-panel-compound

## Purpose

Defines the compound-component API for `Panel` and `PanelGroup`, where sub-components share state via typed context and distinct behaviors are expressed as distinct sub-components rather than boolean props.

## Requirements

### Requirement: Panel compound component API

The `Panel` export SHALL be a namespace object of sub-components sharing state via a typed context. Sub-components SHALL NOT accept boolean props to alter their behavior; distinct behaviors SHALL be expressed as distinct sub-components. Sub-components SHALL use the `use()` hook (not `useContext`) to access shared context.

The `Panel` namespace SHALL expose at minimum:

| Sub-component          | Responsibility                                                 |
| ---------------------- | -------------------------------------------------------------- |
| `Panel.Provider`       | Injects `{ state: { id, title }, actions, meta }` into context |
| `Panel.Frame`          | Outer container; flex column, full height                      |
| `Panel.Header`         | 24px bar; flex row; left: title area, right: toolbar area      |
| `Panel.Title`          | Reads title from context; renders truncated label              |
| `Panel.Toolbar`        | Right-side flex row in header; renders children (buttons)      |
| `Panel.Toolbar.Button` | Icon button with tooltip; receives `icon`, `label`, `onClick`  |
| `Panel.Content`        | Flex-grow scroll container for panel body content              |

#### Scenario: Sub-components access shared context

- **WHEN** `Panel.Title` renders inside `Panel.Provider`
- **THEN** it displays the title injected by the provider without the title being passed as a prop

#### Scenario: No boolean props

- **WHEN** reviewing the `Panel` API surface
- **THEN** no boolean props exist on any sub-component; variant behavior is expressed via distinct sub-components or children composition

### Requirement: PanelGroup compound component API

The `PanelGroup` export SHALL be a namespace object. It SHALL provide display-mode-specific sub-components rather than a single component with a mode boolean prop. Each display mode composes different sub-components.

The `PanelGroup` namespace SHALL expose at minimum:

| Sub-component             | Used by mode | Responsibility                                                        |
| ------------------------- | ------------ | --------------------------------------------------------------------- |
| `PanelGroup.Provider`     | all          | Injects `{ displayMode, activeTabId, direction, actions }`            |
| `PanelGroup.Split`        | split        | Wraps children in resizable panel group with resize handles           |
| `PanelGroup.TabBar`       | tabbar-\*    | Tab strip container; position (top/bottom) determined by display mode |
| `PanelGroup.TabBar.Tab`   | tabbar-\*    | Single tab; reads panel title from panel registry; activates on click |
| `PanelGroup.Sidebar`      | sidebar      | Vertical accordion list container                                     |
| `PanelGroup.Sidebar.Item` | sidebar      | Single sidebar item; reads panel title; expands/collapses on click    |
| `PanelGroup.Panels`       | all          | Renders active panel (tabbar/sidebar) or all panels (split)           |

#### Scenario: Split mode composition

- **WHEN** a group uses split mode
- **THEN** the consumer composes `PanelGroup.Provider` + `PanelGroup.Split` and the children render with resize handles; no tab bar or sidebar renders

#### Scenario: Tabbar mode composition

- **WHEN** a group uses tabbar-top mode
- **THEN** the consumer composes `PanelGroup.Provider` + `PanelGroup.TabBar` + `PanelGroup.Panels`; only the active panel mounts

#### Scenario: Sidebar mode composition

- **WHEN** a group uses sidebar mode
- **THEN** the consumer composes `PanelGroup.Provider` + `PanelGroup.Sidebar` + `PanelGroup.Panels`; clicking a sidebar item changes the active panel

### Requirement: Panel toolbar button accessibility

Every `Panel.Toolbar.Button` SHALL have an accessible label. The label SHALL be visually presented as a tooltip on hover. The button SHALL be keyboard-focusable and activatable.

#### Scenario: Tooltip on hover

- **WHEN** the user hovers over a toolbar button
- **THEN** a tooltip showing the button's label appears

#### Scenario: Keyboard activation

- **WHEN** a toolbar button has focus and the user presses Enter or Space
- **THEN** the button's `onClick` handler fires

### Requirement: No component definitions inside components

All sub-components SHALL be defined at module level, not inside other components or render functions. Inline component definitions cause remounts on every render.

#### Scenario: Stable component identity

- **WHEN** a parent component re-renders
- **THEN** sub-components of `Panel` and `PanelGroup` maintain stable identity and do not remount
