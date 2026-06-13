## MODIFIED Requirements

### Requirement: Add surface to session

Adding a surface to a session SHALL be a divergence of the session's launch spec. The orchestrator
SHALL append a launch item, mint a placement unique within the session, create the surface bound to
that placement, and record the `session_id` on the surface. The caller supplies a `session_id` and
the new item's target. The orchestrator SHALL reject the request if the `session_id` does not exist.
The minted placement SHALL be returned to the caller so the UI can bind the acting panel to it.

#### Scenario: Spawn appends a launch item, mints a placement, and creates the surface

- **WHEN** an add-surface request supplies a valid `session_id` and a target
- **THEN** the orchestrator appends a launch item, mints a placement, creates the surface, records the `session_id`, and returns the placement

#### Scenario: Unknown session is rejected

- **WHEN** an add-surface request supplies a `session_id` that does not exist
- **THEN** the orchestrator returns a typed error and no item or surface is added

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

### Requirement: Session resume after restart

On orchestrator startup the orchestrator SHALL query the store for all non-archived sessions that have non-archived surfaces and SHALL reconnect each surface to the daemon by its `surface_id`. For each resumed surface the orchestrator SHALL expose its placement so the UI binds it to the panel at that placement. Sessions and surfaces that were active at shutdown SHALL be available to clients without requiring a new session creation request.

#### Scenario: All of a session's surfaces reconnect by placement on startup

- **WHEN** the orchestrator restarts and the store contains a session with non-archived surfaces at two distinct placements
- **THEN** both surfaces are reconnected to the daemon by their `surface_id` and each is exposed with its placement so the UI binds it to the right panel

#### Scenario: Archived sessions not resumed

- **WHEN** the orchestrator restarts and a session is archived
- **THEN** that session's surfaces are not reconnected
