# layout-persistence Specification

## Purpose
Server-side persistence of a session's panel-tree layout in the `layout_json` column, replacing browser local storage as the backend. Covers storing layout on request, restoring it (including empty-leaf and null-layout fallback) on session open, keeping it in sync after structural panel mutations, and discarding legacy local-storage layout keys on UI init.
## Requirements
### Requirement: Per-session layout stored in the product store

The orchestrator SHALL store the panel-tree layout for each session as a versioned JSON blob in the `layout_json` column of the session record. The UI SHALL NOT use the browser's local storage as the persistence backend for layout after this change takes effect. The orchestrator SHALL accept a store-layout request carrying a `session_id` and a layout blob, persist it, and return success.

#### Scenario: Layout persisted for a session

- **WHEN** the UI sends a store-layout request with a valid `session_id` and a layout blob
- **THEN** the session record's `layout_json` is updated to the supplied blob

#### Scenario: Layout for unknown session rejected

- **WHEN** a store-layout request supplies a `session_id` that does not exist
- **THEN** the orchestrator returns a typed not-found error

### Requirement: Layout restored on session open

The orchestrator SHALL return the stored `layout_json` blob when a get-session or open-session request is made for a session that has a persisted layout. The panel tree (the stored geometry) carries a placement binding per leaf and is the per-session record the UI restores on open: a leaf bound to a placement SHALL render the surface resolved by `(session, placement)`, and an empty (unbound) leaf SHALL be kept as durable geometry. When no layout has been stored (`layout_json` is NULL) the UI SHALL render a single empty leaf and SHALL NOT inherit the previously-open session's tree. Spawn and close each write both the stored geometry and the launch spec, so the tree and the spec stay in agreement without a reconciliation pass on open. (A spec-authoritative reconcile -- adding a leaf for a spec placement that has no leaf, dropping a leaf whose placement is absent from the spec -- is a deferred follow-up for cross-client/external divergence; it needs a session-placements read.)

#### Scenario: Stored layout restored on open

- **WHEN** the UI requests a session that has a persisted layout
- **THEN** the response includes the `layout_json` blob exactly as stored and each bound leaf resolves its surface by `(session, placement)`

#### Scenario: Empty leaf is kept on open

- **WHEN** the stored geometry has an empty panel with no bound placement
- **THEN** the UI keeps that panel as geometry

#### Scenario: Null layout falls back to an empty leaf

- **WHEN** the UI requests a session whose `layout_json` is NULL
- **THEN** the UI renders a single empty leaf and does not inherit the previous session's tree

### Requirement: Versioned layout clean cutover

The UI SHALL serialize panel geometry as `{ "version": 1, "root": <panel-tree> }` and SHALL accept only that versioned envelope with valid split-group sizes. An unversioned, unsupported-version, or invalid non-null blob SHALL produce an incompatible-layout error. The UI SHALL NOT infer sizes, migrate the blob, overwrite it, or render a replacement tree. A null layout SHALL continue to render the single-empty-leaf default.

#### Scenario: Versioned nested geometry restores

- **WHEN** a session stores a version-1 layout containing nested split groups with valid sizes
- **THEN** the UI restores the complete tree and each group's independent sizes

#### Scenario: Unversioned layout is rejected

- **WHEN** a session stores the previous unversioned panel-tree blob
- **THEN** the UI renders a blocking incompatible-layout alert without inferring, replacing, or overwriting geometry, while the existing sidebar remains available to discard the development session

### Requirement: Layout updated on panel mutation

The UI SHALL send a store-layout request to the orchestrator after every panel mutation that changes persisted state, including split, close, mode change, content assignment, completed divider resize, and divider reset. The in-memory panel tree and the persisted layout SHALL remain consistent: a page reload SHALL produce the same panel tree and child sizes as were present before the reload.

#### Scenario: Layout written after split

- **WHEN** the user splits a panel
- **THEN** the UI sends a store-layout request with the updated tree before the next render cycle completes

#### Scenario: Layout written after resize

- **WHEN** the user completes a divider drag or resets a divider
- **THEN** the UI sends a store-layout request containing the updated normalized sizes

#### Scenario: Reload restores panel tree

- **WHEN** the user reloads the page while a session with a persisted layout is active
- **THEN** the panel tree and every split group's child sizes are initialized from the persisted versioned layout

### Requirement: Local storage layout migration

When the UI initialises and detects a legacy global layout key in browser local storage, it SHALL discard that key and fall back to the server-persisted layout for the current session (or the default layout if none is stored). It SHALL NOT write the legacy key back.

#### Scenario: Legacy local-storage key discarded

- **WHEN** the UI initialises and finds a legacy layout key in browser local storage
- **THEN** it removes that key, loads the server-persisted layout for the current session, and does not write the legacy key again

