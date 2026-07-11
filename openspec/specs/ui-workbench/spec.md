# ui-workbench Specification

## Purpose
TBD - created by archiving change ux-ui-overhaul. Update Purpose after archive.
## Requirements
### Requirement: Workbench regions

The shell SHALL compose five chrome regions around the panel-area content outlet: a title
bar (top), an activity bar (far left icon strip), a primary sidebar (left, hosting the
active sidebar view), a bottom panel (below the content outlet), and a status bar
(bottom). The sidebar and bottom panel SHALL be independently hideable and drag-resizable
within defined min/max bounds; a hidden region SHALL occupy no layout space and its resize
handle SHALL be absent. The activity bar, title bar, and status bar SHALL always be
visible. There SHALL be no right dock.

#### Scenario: Hidden region reclaims space

- **WHEN** the bottom panel is hidden
- **THEN** the bottom panel and its resize handle occupy no vertical space and the content
  area extends to the status bar

#### Scenario: Regions are independently controlled

- **WHEN** the user hides the sidebar
- **THEN** the bottom panel visibility is unchanged

#### Scenario: A visible region can be resized

- **WHEN** the user drags the handle between the content area and a visible region
- **THEN** that region resizes within its min/max bounds and the content area takes the
  remaining space

### Requirement: Activity bar switches sidebar views

The activity bar SHALL render one icon button per registered sidebar view (Sessions,
Search, Commands, Templates), with the active view visually indicated. Activating a view's
icon SHALL show that view in the sidebar; activating the active view's icon SHALL toggle
the sidebar's visibility. Each icon SHALL expose its view title (tooltip and accessible
name).

#### Scenario: Switching views

- **WHEN** the Sessions view is active and the user activates the Commands icon
- **THEN** the sidebar shows the command library view and the Commands icon is marked
  active

#### Scenario: Reactivating toggles the sidebar

- **WHEN** the Sessions view is active and visible and the user activates the Sessions
  icon
- **THEN** the sidebar hides; activating it again shows the Sessions view

### Requirement: Bottom panel hosts Logs and Notifications tabs

The bottom panel SHALL present a tab strip with Logs and Notifications tabs. The Logs tab
SHALL render the log viewer (same capability as the log-viewer route). The Notifications
tab SHALL render the notification feed. Opening the bottom panel via a tab-specific
affordance (status bar bell, health logs link, command) SHALL activate that tab.

#### Scenario: Tab switching

- **WHEN** the bottom panel shows the Logs tab and the user activates the Notifications
  tab
- **THEN** the notification feed replaces the log viewer within the bottom panel

#### Scenario: Bell opens Notifications tab

- **WHEN** the bottom panel is hidden and the user activates the status bar bell
- **THEN** the bottom panel opens with the Notifications tab active

### Requirement: Status bar

The status bar SHALL span the full window width at the bottom of the shell and present:
the aggregate service-health item and the active workspace and session context on the
left; the notification bell with unread badge and a settings shortcut on the right. Status
bar items SHALL be projections of commands tagged for the `statusbar` surface where they
trigger actions.

#### Scenario: Context reflects the active session

- **WHEN** a session is active in the panel area
- **THEN** the status bar shows the active workspace and session titles

#### Scenario: Settings shortcut opens the settings editor

- **WHEN** the user activates the status bar settings item
- **THEN** the settings editor opens in the panel area

### Requirement: Workbench state persists

The active sidebar view, sidebar visibility and size, and bottom panel visibility, size,
and active tab SHALL persist across restarts via the settings store, restoring on launch
and defaulting to a defined initial state when no value is stored.

#### Scenario: State survives restart

- **WHEN** the user switches to the Templates view, resizes the sidebar, opens the bottom
  panel on Logs, and restarts the application
- **THEN** the workbench restores the Templates view, the sidebar size, and the open Logs
  tab

#### Scenario: First-launch defaults

- **WHEN** the application launches with no stored workbench state
- **THEN** the Sessions view is active, the sidebar is visible, and the bottom panel is
  hidden

### Requirement: Native menu accelerators

On the desktop host, the application menu SHALL expose accelerators for at minimum: new
project, new session, new terminal surface, close surface, and switch session, labeled
with the platform's modifier convention. Menu accelerators SHALL fire even while a
terminal surface holds keyboard focus and SHALL route through the same command ids as the
palette.

#### Scenario: Accelerator fires with terminal focus

- **WHEN** a terminal surface holds keyboard focus and the user presses the new-terminal
  accelerator
- **THEN** the new-terminal command runs

#### Scenario: Platform-correct labels

- **WHEN** the application menu renders on macOS
- **THEN** accelerators display with the command-key convention rather than a Ctrl label

### Requirement: Chrome stays responsive under load

Workbench chrome interactions (view switching, region resize, tab switching) SHALL remain
responsive with multiple sessions and running surfaces, and repeated session switching
SHALL NOT grow memory without bound.

#### Scenario: Chrome interaction during streaming output

- **WHEN** multiple terminal surfaces stream output and the user resizes the sidebar
- **THEN** the resize tracks the pointer without visible stalls

