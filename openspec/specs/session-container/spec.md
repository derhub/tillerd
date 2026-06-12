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

The orchestrator SHALL associate an existing surface record with a session by recording the `session_id` on the surface. The caller supplies a `session_id` and a `surface_id`. The orchestrator SHALL reject the request if either id does not exist or the surface is already associated with a different session.

#### Scenario: Surface associated with session

- **WHEN** an add-surface request supplies a valid `session_id` and a valid `surface_id` with no current session association
- **THEN** the surface record's `session_id` is set to the supplied value

#### Scenario: Surface already in another session is rejected

- **WHEN** an add-surface request supplies a `surface_id` that is already associated with a different `session_id`
- **THEN** the orchestrator returns a typed conflict error and the surface is not moved

### Requirement: Remove surface from session

The orchestrator SHALL soft-delete a surface record when a remove-surface request supplies a `session_id` and a `surface_id`. The surface's PTY SHALL NOT be terminated by this operation; only the surface row is soft-deleted.

#### Scenario: Remove soft-deletes surface row

- **WHEN** a remove-surface request supplies a valid `session_id` and a valid `surface_id` belonging to that session
- **THEN** the surface record's `deleted_at` is set and the surface no longer appears in active surface lists

#### Scenario: PTY not terminated on remove

- **WHEN** a surface is removed from a session
- **THEN** the underlying pseudo-terminal process is not killed

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

On orchestrator startup the orchestrator SHALL query the store for all non-archived sessions that have non-archived surfaces and SHALL reconnect each surface to the daemon by its `surface_id`. Sessions and surfaces that were active at shutdown SHALL be available to clients without requiring a new session creation request.

#### Scenario: Active surfaces reconnected on startup

- **WHEN** the orchestrator restarts and the store contains a session with a non-archived surface
- **THEN** that surface is reconnected to the daemon by its `surface_id` and is available to clients

#### Scenario: Archived sessions not resumed

- **WHEN** the orchestrator restarts and a session is archived
- **THEN** that session's surfaces are not reconnected

