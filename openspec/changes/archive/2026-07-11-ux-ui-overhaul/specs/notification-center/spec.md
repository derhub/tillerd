# notification-center

## MODIFIED Requirements

### Requirement: In-app notification center

The system SHALL present a bell control in the status bar that opens the bottom panel's
Notifications tab, listing recent notifications most recent first, each showing its
message, timestamp, and session context when present. The list SHALL never block the rest
of the app.

#### Scenario: Opening the center lists recent notifications

- **WHEN** the user activates the bell control
- **THEN** the bottom panel opens on the Notifications tab showing recent notifications
  most-recent-first with message, time, and session context

#### Scenario: Empty feed shows an empty state

- **WHEN** the user activates the bell control and no notifications have been recorded
- **THEN** an empty state is shown rather than an error or a blank panel

## ADDED Requirements

### Requirement: Notification management actions

The Notifications tab SHALL let the user mark a notification read, mark all read,
disregard (remove) a notification, disregard all, and snooze a notification for a chosen
duration. A snoozed notification SHALL leave the unread count until its snooze elapses.

#### Scenario: Disregarding removes the entry

- **WHEN** the user disregards a notification
- **THEN** it disappears from the feed and does not return on restart

#### Scenario: Snoozing defers the notification

- **WHEN** the user snoozes an unread notification
- **THEN** it stops counting as unread and reappears as unread after the snooze elapses
