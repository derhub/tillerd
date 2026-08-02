## MODIFIED Requirements

### Requirement: Workbench regions

The shell SHALL compose six chrome regions around the panel-area content outlet: a title bar (top), an activity bar (far left icon strip), a primary sidebar (left, hosting the active sidebar view), a bottom panel (below the content outlet), a right dock (right), and a status bar (bottom). The sidebar, bottom panel, and right dock SHALL be independently hideable and drag-resizable within defined min/max bounds; a hidden region SHALL occupy no layout space and its resize handle SHALL be absent. The activity bar, title bar, and status bar SHALL always be visible. The right dock SHALL be chrome only: it SHALL NOT create a surface, claim a placement, or alter the panel tree or session surface ownership.

#### Scenario: Hidden region reclaims space

- **WHEN** the bottom panel is hidden
- **THEN** the bottom panel and its resize handle occupy no vertical space and the content area extends to the status bar

#### Scenario: Regions are independently controlled

- **WHEN** the user hides the sidebar
- **THEN** the bottom panel and right dock visibility are unchanged

#### Scenario: A visible region can be resized

- **WHEN** the user drags the handle between the content area and a visible region
- **THEN** that region resizes within its min/max bounds and the content area takes the remaining space

#### Scenario: Right dock hosts a view

- **WHEN** the user opens an available diff, review, or agent view
- **THEN** the workbench renders it in the right dock without creating a surface or changing a placement record
