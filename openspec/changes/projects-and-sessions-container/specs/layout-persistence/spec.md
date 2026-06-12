## ADDED Requirements

### Requirement: Per-session layout stored in the product store

The orchestrator SHALL store the panel-tree layout for each session as a versioned JSON blob in the `layout_json` column of the session record. The UI SHALL NOT use the browser's local storage as the persistence backend for layout after this change takes effect. The orchestrator SHALL accept a store-layout request carrying a `session_id` and a layout blob, persist it, and return success.

#### Scenario: Layout persisted for a session

- **WHEN** the UI sends a store-layout request with a valid `session_id` and a layout blob
- **THEN** the session record's `layout_json` is updated to the supplied blob

#### Scenario: Layout for unknown session rejected

- **WHEN** a store-layout request supplies a `session_id` that does not exist
- **THEN** the orchestrator returns a typed not-found error

### Requirement: Layout restored on session open

The orchestrator SHALL return the stored `layout_json` blob when a get-session or open-session request is made for a session that has a persisted layout. When no layout has been stored (`layout_json` is NULL) the orchestrator SHALL return a null or absent field and the UI SHALL apply its default layout.

#### Scenario: Stored layout returned on open

- **WHEN** the UI requests a session that has a persisted layout
- **THEN** the response includes the `layout_json` blob exactly as stored

#### Scenario: Null returned when no layout stored

- **WHEN** the UI requests a session whose `layout_json` is NULL
- **THEN** the response carries a null or absent `layout_json` field and the UI renders its default layout

### Requirement: Layout updated on panel mutation

The UI SHALL send a store-layout request to the orchestrator after every structural panel mutation (split, close, mode change, content assignment). The in-memory panel tree and the persisted layout SHALL remain consistent: a page reload SHALL produce the same panel tree as was present before the reload.

#### Scenario: Layout written after split

- **WHEN** the user splits a panel
- **THEN** the UI sends a store-layout request with the updated tree before the next render cycle completes

#### Scenario: Reload restores panel tree

- **WHEN** the user reloads the page while a session with a persisted layout is active
- **THEN** the panel tree is initialized from the persisted layout, not from browser local storage

### Requirement: Local storage layout migration

When the UI initialises and detects a legacy global layout key in browser local storage, it SHALL discard that key and fall back to the server-persisted layout for the current session (or the default layout if none is stored). It SHALL NOT write the legacy key back.

#### Scenario: Legacy local-storage key discarded

- **WHEN** the UI initialises and finds a legacy layout key in browser local storage
- **THEN** it removes that key, loads the server-persisted layout for the current session, and does not write the legacy key again
