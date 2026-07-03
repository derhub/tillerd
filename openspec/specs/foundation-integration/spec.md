# foundation-integration

## Purpose

The three foundation slices — the persistent store, the state model, and the client engine —
compose into one coherent whole. An entity created in the app keeps its identity across a
session switch and a full window reload, and windows stay coherent through the server-state
cache rather than a file merge. This capability is the cross-cutting integration guarantee the
per-slice capabilities (session, surface, layout, view pointers, multi-window, workspace
activity) do not each own on their own; it is proven end to end by a single continuous e2e
journey.

## Requirements

### Requirement: Created entities persist and appear without a manual refresh

The system SHALL persist a newly created project, session, and surface, and SHALL make each
visible in the creating window immediately, driven by the server-state cache rather than a
manual refresh.

#### Scenario: Create project, session, and surface

- **WHEN** a project, a session within it, and a terminal surface are created in one window
- **THEN** each entity is persisted to the store
- **AND** each appears in that window with no manual refresh

### Requirement: A session and its surface restore on revisit

The system SHALL restore a session and its surface from the store when the user navigates away
and back, preserving the surface identity so its content is not lost.

#### Scenario: Switch away and back to a session

- **WHEN** the user switches to another session and then navigates back
- **THEN** the original session's own surface is restored with the same surface identity
- **AND** it is not replaced by another session's surface or a freshly spawned one

### Requirement: A deep session route survives a window reload

The system SHALL restore the current deep session route, its rendered content, and its surface
after a full window reload, rather than falling back to a default view.

#### Scenario: Reload at a deep session route

- **WHEN** the window reloads while on a deep session route
- **THEN** the same deep route is served
- **AND** the project and the session's surface are restored from the store with the surface
  identity intact

### Requirement: Windows stay coherent through the server-state cache

The system SHALL keep multiple windows coherent by invalidating the matching query in the other
windows when a write occurs, driven by a cross-window cache-invalidation broadcast — not by
re-reading files and not by a conflict prompt.

#### Scenario: A write in one window reaches the others

- **WHEN** a write occurs in one window while the same target is open in another
- **THEN** the other window reflects the change without a manual refresh
- **AND** no file-merge or conflict-resolution prompt is presented
