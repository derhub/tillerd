## ADDED Requirements

### Requirement: Lifecycle signals become user notifications

The system SHALL turn user-relevant lifecycle signals into notifications: a surface
starting, a surface stopping or exiting, a surface error, a service becoming available or
unavailable, and an orchestrator status change. Each notification SHALL carry a category, a
human-readable message, a timestamp, and — where the signal originates from a surface — the
session or surface it concerns.

#### Scenario: Surface exit raises a notification

- **WHEN** a surface exits
- **THEN** a notification is recorded with the surface's session context, the exit qualifier
  in its message, and the time it occurred

#### Scenario: A service becoming unavailable raises a notification

- **WHEN** a supervised service transitions from available to unavailable between two health
  snapshots
- **THEN** a notification is recorded naming the service and its new state

#### Scenario: A service recovering raises a notification

- **WHEN** a supervised service transitions from unavailable back to available
- **THEN** a notification is recorded naming the service and its restored state

#### Scenario: An unchanged health snapshot raises nothing

- **WHEN** consecutive health snapshots report the same state for every service
- **THEN** no service notification is recorded

### Requirement: The feed is global, independent of the focused surface

The notification feed SHALL capture lifecycle events from every surface, including surfaces
that are not currently displayed, so a background surface's stop or error is never lost.

#### Scenario: A background surface error is captured

- **WHEN** a surface that is not currently displayed reports an error
- **THEN** the error is recorded in the notification feed

### Requirement: In-app notification center

The system SHALL present a bell control in the app chrome that opens a list of recent
notifications, most recent first, each showing its message, timestamp, and session context
when present. The list SHALL be read-only and never block the rest of the app.

#### Scenario: Opening the center lists recent notifications

- **WHEN** the user activates the bell control
- **THEN** recent notifications are shown most-recent-first with message, time, and session
  context

#### Scenario: Empty feed shows an empty state

- **WHEN** the user activates the bell control and no notifications have been recorded
- **THEN** an empty state is shown rather than an error or a blank panel

### Requirement: Rich content and actions

A notification MAY carry an optional title, an optional longer detail, a severity, and a
list of actions (each a label and an in-app target). The center SHALL render these when
present and SHALL degrade gracefully when they are absent or when the category is one it does
not specifically recognise.

#### Scenario: Title, detail, and severity render when present

- **WHEN** a notification with a title, detail, and a non-default severity is shown
- **THEN** the title, detail, and a severity indication are all presented

#### Scenario: Missing title falls back to a category label

- **WHEN** a notification without a title is shown
- **THEN** a category-derived label is presented in place of the title

#### Scenario: Actions render as activatable controls

- **WHEN** a notification carrying one or more actions is shown and the user activates an action
- **THEN** the application navigates to that action's target without a full page reload

#### Scenario: An unrecognised category still renders

- **WHEN** a notification whose category is not one of the known kinds is recorded
- **THEN** it is still listed with its message rather than dropped or erroring

### Requirement: Unread badge

The bell SHALL show an unread count for notifications recorded since the center was last
opened, and the count SHALL clear when the center is opened.

#### Scenario: New notification increments the unread count

- **WHEN** a notification is recorded while the center is closed
- **THEN** the bell's unread count increases by one

#### Scenario: Opening the center clears the unread count

- **WHEN** the user opens the center while there are unread notifications
- **THEN** the unread count returns to zero

### Requirement: Native OS banner for background events

The system SHALL raise a native operating-system banner for a notification only when the
application window is not focused. The banner SHALL be dismissable and, when activated, SHALL
bring the application to the foreground (precise in-app routing is the in-app feed's role).

#### Scenario: Banner raised when the window is unfocused

- **WHEN** a notification is recorded while the application window is not focused
- **THEN** a native OS banner is raised for it

#### Scenario: No banner when the window is focused

- **WHEN** a notification is recorded while the application window is focused
- **THEN** no native OS banner is raised, and the notification still appears in the in-app feed

### Requirement: Click-through navigation

Activating a notification that concerns a session in the in-app list SHALL navigate to that
session within the running application without reloading it.

#### Scenario: Activating a session notification navigates in-app

- **WHEN** the user activates a notification that names a session
- **THEN** the application navigates to that session without a full page reload

### Requirement: Bounded durable history

The notification history SHALL be persisted durably, SHALL survive application restarts, and
SHALL be bounded to a fixed maximum by discarding the oldest entries first.

#### Scenario: Oldest entries are trimmed at the bound

- **WHEN** more notifications are recorded than the history bound allows
- **THEN** the oldest entries are discarded and the newest are retained up to the bound

#### Scenario: History survives a restart

- **WHEN** notifications are recorded, then the application is quit and started again
- **THEN** the previously recorded notifications are present in the feed, most recent first

### Requirement: Sole user-facing feedback channel

The notification center SHALL be the only user-facing feedback channel for these events; the
system SHALL NOT raise transient in-app toast popups.

#### Scenario: Events surface through the center, not a toast

- **WHEN** a user-relevant lifecycle event occurs
- **THEN** it is recorded in the notification center and no transient toast is shown
