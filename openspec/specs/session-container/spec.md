# session-container Specification

## Purpose
TBD - created by archiving change projects-and-sessions-container. Update Purpose after archive.
## Requirements
### Requirement: Session creation under a project

The orchestrator SHALL create a session under a specified project. The caller SHALL supply a `project_id`; if none is supplied the orchestrator SHALL assign the session to the Unfiled project. On creation the orchestrator SHALL persist a session row with a generated `session_id`, the resolved `project_id`, and a `title` derived according to the `title_source` strategy. The orchestrator SHALL return the new `session_id`.

#### Scenario: Session created under supplied project

- **WHEN** a create-session request supplies a valid `project_id`
- **THEN** the session row is persisted with that `project_id` and the new `session_id` is returned

#### Scenario: Session defaults to Unfiled when project omitted

- **WHEN** a create-session request omits `project_id`
- **THEN** the session row is persisted with `project_id` equal to the Unfiled project id

### Requirement: Session title inference

The orchestrator SHALL infer a session title according to the `title_source` field. Valid strategies are `agent-title` (use the title reported by the agent on session completion), `branch` (use the current git branch of the session's root path), `both` (concatenate branch and agent title), and `custom` (use the caller-supplied `title` verbatim). When inference is not yet possible at creation time the title SHALL be stored as an empty string and updated when the source becomes available.

#### Scenario: Branch strategy sets title to git branch

- **WHEN** a session is created with `title_source = "branch"` and the root path is a git repository on branch `feat/x`
- **THEN** the persisted title is `feat/x`

#### Scenario: Custom strategy uses supplied title

- **WHEN** a session is created with `title_source = "custom"` and `title = "My session"`
- **THEN** the persisted title is `My session`

#### Scenario: Agent-title strategy deferred until available

- **WHEN** a session is created with `title_source = "agent-title"` and no agent title is yet available
- **THEN** the persisted title is an empty string until the agent title is reported

### Requirement: Session rename

The orchestrator SHALL update a session's title and set `title_source` to `custom` when a rename request supplies a new title. The updated title SHALL be reflected in subsequent list and get responses.

#### Scenario: Rename updates title and source

- **WHEN** a rename-session request supplies a valid `session_id` and a non-empty title
- **THEN** the session record's `title` is updated to the supplied value and `title_source` is set to `custom`

#### Scenario: Rename unknown session returns error

- **WHEN** a rename-session request supplies a `session_id` that does not exist
- **THEN** the orchestrator returns a typed not-found error

### Requirement: Session listing

The orchestrator SHALL return the list of non-archived sessions. When an optional `project_id` filter is supplied the response SHALL include only sessions belonging to that project. Each entry SHALL include `session_id`, `project_id`, `title`, `title_source`, and `created_at`.

#### Scenario: Unfiltered list returns all active sessions

- **WHEN** two active sessions exist under different projects
- **THEN** the list response contains both

#### Scenario: Filtered list returns sessions for that project only

- **WHEN** a list request supplies a `project_id`
- **THEN** the response contains only sessions whose `project_id` matches

#### Scenario: Archived sessions excluded

- **WHEN** one session is active and another is archived
- **THEN** only the active session appears in the list

### Requirement: Add surface to session

Adding a surface to a session SHALL be a divergence of the session's launch spec. The orchestrator
SHALL append a launch item, mint a placement unique within the session, record it on the session
spec, and return the placement to the caller. The caller supplies a `session_id` and the new item's
target. The orchestrator SHALL reject the request if the `session_id` does not exist. The renderer
then creates the surface at the returned placement -- it owns the surface byte channel -- and binds
the acting panel to it; surface creation resolves or creates by `(session, placement)`.

#### Scenario: Spawn appends a launch item and returns the minted placement

- **WHEN** an add-surface request supplies a valid `session_id` and a target
- **THEN** the orchestrator appends a launch item, mints a placement unique within the session, and returns it; the renderer then creates the surface at that placement

#### Scenario: Unknown session is rejected

- **WHEN** an add-surface request supplies a `session_id` that does not exist
- **THEN** the orchestrator returns a typed error and no item is added

### Requirement: Remove surface from session

Removing a surface SHALL be a divergence of the session's launch spec and a hard remove. The
orchestrator SHALL remove the surface's launch item from the session spec, delete the surface row,
and terminate the surface's pseudo-terminal, when a remove-surface request supplies a `session_id`
and a `surface_id`. A removed surface SHALL NOT be resumed on a later start. This is distinct from
session archive, which soft-deletes surfaces and preserves their pseudo-terminals for restore.

#### Scenario: Remove drops the launch item and terminates the PTY

- **WHEN** a remove-surface request supplies a valid `session_id` and a valid `surface_id` belonging to that session
- **THEN** the surface's launch item is removed from the session spec, the surface row is removed, and the pseudo-terminal is terminated

#### Scenario: Removed surface is not resumed

- **WHEN** the host restarts after a surface was removed
- **THEN** that surface is not reconnected and does not reappear in the session

### Requirement: Session archive (soft-delete) with cascade

The orchestrator SHALL soft-delete a session by setting its `deleted_at` timestamp and cascading the soft-delete to all surfaces belonging to that session. After archiving, the session and its surfaces SHALL not appear in active list responses.

#### Scenario: Archived session excluded from list

- **WHEN** a session is archived
- **THEN** it does not appear in the active session list

#### Scenario: Cascade soft-deletes session surfaces

- **WHEN** a session is archived and it has active surfaces
- **THEN** all surface records for that session are soft-deleted in the same operation

### Requirement: Session hard-delete

The orchestrator SHALL permanently remove an already-archived session and all of its archived surface records. Hard-delete SHALL be rejected if the session is not already archived.

#### Scenario: Hard-delete removes session and surface rows

- **WHEN** a hard-delete-session request targets a session with `deleted_at` set
- **THEN** the session row and all child surface rows are permanently removed

#### Scenario: Hard-delete on active session rejected

- **WHEN** a hard-delete-session request targets a session that is not archived
- **THEN** the orchestrator returns a typed error and no rows are removed

### Requirement: Session resume after restart

On orchestrator startup the orchestrator SHALL query the store for all non-archived sessions that have non-archived surfaces and SHALL reconnect each surface to the daemon by its `surface_id`. For each resumed surface the orchestrator SHALL expose its placement so the UI binds it to the panel at that placement. Sessions and surfaces that were active at shutdown SHALL be available to clients without requiring a new session creation request.

#### Scenario: All of a session's surfaces reconnect by placement on startup

- **WHEN** the orchestrator restarts and the store contains a session with non-archived surfaces at two distinct placements
- **THEN** both surfaces are reconnected to the daemon by their `surface_id` and each is exposed with its placement so the UI binds it to the right panel

#### Scenario: Archived sessions not resumed

- **WHEN** the orchestrator restarts and a session is archived
- **THEN** that session's surfaces are not reconnected

