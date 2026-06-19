# session-container Specification — DELTA

Change: `snapshot-tree-storage`

---

## MODIFIED Requirements

### Requirement: Session creation under a project

The orchestrator SHALL create a session under a specified project. The caller SHALL supply a `project_id`; if none is supplied the orchestrator SHALL assign the session to the Unfiled project. On creation the orchestrator SHALL write a `session.json` file into the snapshot tree under the project's slug directory (as owned by the `snapshot-tree-store` capability), recording the generated `session_id`, the resolved `project_id`, and a `title` derived according to the `title_source` strategy. The orchestrator SHALL return the new `session_id`.

#### Scenario: Session created under supplied project

- **WHEN** a create-session request supplies a valid `project_id`
- **THEN** a `session.json` is written under that project's slug directory and the new `session_id` is returned

#### Scenario: Session defaults to Unfiled when project omitted

- **WHEN** a create-session request omits `project_id`
- **THEN** the `session.json` is written under the Unfiled project's slug directory

---

### Requirement: Session archive (soft-delete) with cascade

The orchestrator SHALL archive a session by moving its subtree (including all surface data beneath it) to `.archive/` in one atomic move, as provided by the `snapshot-tree-store` capability. The directory move IS the cascade — no separate surface records need to be updated. After archiving, the session and its surfaces SHALL not appear in active list responses.

#### Scenario: Archived session excluded from list

- **WHEN** a session is archived
- **THEN** it does not appear in the active session list

#### Scenario: Cascade moves session surfaces

- **WHEN** a session is archived and it has surfaces
- **THEN** the entire session subtree, including all surface data, is moved to `.archive/` in the same atomic operation

#### Scenario: Surfaces not resumed after archive

- **WHEN** the host restarts after a session is archived
- **THEN** the session's surfaces are not reconnected and do not reappear in the active session
