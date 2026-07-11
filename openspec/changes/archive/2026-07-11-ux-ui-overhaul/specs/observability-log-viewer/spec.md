# observability-log-viewer

## MODIFIED Requirements

### Requirement: Global log-viewer route

The application SHALL present the log viewer at its own global route, scoped to the app
shell, and the same log view SHALL render as the bottom panel's Logs tab. The log view
SHALL NOT be a session surface and SHALL NOT occupy a placement in any session's panel
tree. Both hosts SHALL honor the service filter (the route via its query parameter, the
tab via the health panel's logs link).

#### Scenario: Reaching the log viewer

- **WHEN** the user navigates to the log-viewer route
- **THEN** the application shows the log view in the content area, independent of any
  session

#### Scenario: Logs tab in the bottom panel

- **WHEN** the user opens the bottom panel's Logs tab
- **THEN** the same log view renders inside the bottom panel while the panel area keeps
  its current content

#### Scenario: Not a session surface

- **WHEN** a session is active and the user opens the log-viewer route or the Logs tab
- **THEN** the view is shown as app-shell chrome and consumes no session placement or
  surface
