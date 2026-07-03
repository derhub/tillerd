## ADDED User Stories

### User Story: Foundation slices behave as one across a continuous journey

As a developer trusting the foundation before the UX ship, I want the persistent
store, the state model, and the client engine proven together in a single
continuous flow, so that I have evidence the three slices integrate — not just
that each behaves in isolation.

#### Acceptance Criteria

- **Given** a freshly booted desktop app with no prior domain data
- **When** a project, a session within it, and a terminal surface are created in
  one window
- **Then** each newly created entity is persisted and immediately visible in that
  window without a manual refresh.

- **Given** the created project and session
- **When** the user switches away to another view and back to the session
- **Then** the session and its surface are restored from the persistent store with
  their prior identity intact.

- **Given** the user is on a deep session route
- **When** the window reloads
- **Then** the same deep route and its restored content survive the reload rather
  than falling back to a default view.

- **Given** the project is open in two windows
- **When** a write occurs in one window
- **Then** the other window reflects the change without a manual refresh, driven by
  the shared server-state cache rather than by re-reading files.

## REMOVED User Stories

### User Story: Re-sync conflict prompt for concurrent edits

As a user editing layout across windows, I wanted a re-sync flow that surfaced a
conflict prompt (Override / Force-merge) when concurrent edits diverged, so that I
could resolve a merge by hand.

#### Removal Rationale

The capability is moot as scoped. It assumed the abandoned file-merge storage
world: hand-editable domain files that could diverge and require a three-way
merge. The domain now lives in a single relational store where every mutation is
one atomic transaction, and the server-state cache is the sync axis
(pending / error / stale) — there are no divergent files to merge. Conflict
locking was dropped together with the file-merge model, and multi-window
coherence is already provided by a cross-window cache-invalidation broadcast: a
write in one window invalidates the matching query in the others. No conflict-
prompt UI is built; the roadmap bullet is dropped with a note that these two
superseding decisions cover the concern.
