# desktop-app-data

## Purpose

Native local persistence for the desktop application: a user-preferences store and a session
registry (sessionId -> working directory) the renderer reads and writes over `invoke`, replacing
the server-side registry on the desktop path.

## Requirements

### Requirement: Native store for user preferences

The native core SHALL provide a local store for user preferences that the renderer can read and
write, persisting values across application restarts.

#### Scenario: Persisting a preference

- **WHEN** the renderer writes a user preference and the application is later restarted
- **THEN** the native core returns the previously written value on read

### Requirement: Native session registry

The native core SHALL maintain a registry of sessions and their working directories, and SHALL
reconcile it against the daemon's live sessions on startup, so the renderer can reconnect to a
session that outlived a previous window.

#### Scenario: Reconnecting after the window was closed

- **WHEN** the renderer requests reconnection to a session recorded in the registry that is still
  live in the daemon
- **THEN** the native core supplies the session's working directory for the reconnect

#### Scenario: Reconciling stale registry entries on startup

- **WHEN** the application starts and the registry contains entries with no corresponding live
  daemon session
- **THEN** the native core removes those stale entries
