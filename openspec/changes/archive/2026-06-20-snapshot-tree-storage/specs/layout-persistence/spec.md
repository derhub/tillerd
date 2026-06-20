## MODIFIED Requirements

### Requirement: Per-session layout stored in the product store

The orchestrator SHALL store the panel-tree layout for each session as a versioned JSON blob in the session's `layout.json` file inside the session directory. The file mechanism, placement encoding, and surface-binding shape (`{id, kind, placement, cwd}` per leaf) are owned by the `snapshot-tree-store` capability — see its `Placement-uniqueness enforced by the store` requirement. The UI SHALL NOT use the browser's local storage as the persistence backend for layout after this change takes effect. The orchestrator SHALL accept a store-layout request carrying a `session_id` and a layout blob, persist it, and return success.

#### Scenario: Layout persisted for a session

- **WHEN** the UI sends a store-layout request with a valid `session_id` and a layout blob
- **THEN** the session's `layout.json` file is updated to the supplied blob

#### Scenario: Layout for unknown session rejected

- **WHEN** a store-layout request supplies a `session_id` that does not exist
- **THEN** the orchestrator returns a typed not-found error

### Requirement: Layout restored on session open

The orchestrator SHALL return the stored layout blob when a get-session or open-session request is made for a session that has a persisted `layout.json`. The panel tree (the stored geometry) carries a placement binding per leaf and is the per-session record the UI restores on open: a leaf bound to a placement SHALL render the surface resolved by `(session, placement)`, and an empty (unbound) leaf SHALL be kept as durable geometry. When no layout has been stored (no `layout.json` present) the UI SHALL render a single empty leaf and SHALL NOT inherit the previously-open session's tree. Spawn and close each write both the stored geometry and the launch spec, so the tree and the spec stay in agreement without a reconciliation pass on open. (A spec-authoritative reconcile -- adding a leaf for a spec placement that has no leaf, dropping a leaf whose placement is absent from the spec -- is a deferred follow-up for cross-client/external divergence; it needs a session-placements read.)

#### Scenario: Stored layout restored on open

- **WHEN** the UI requests a session that has a persisted layout
- **THEN** the response includes the layout blob exactly as stored from `layout.json` and each bound leaf resolves its surface by `(session, placement)`

#### Scenario: Empty leaf is kept on open

- **WHEN** the stored geometry has an empty panel with no bound placement
- **THEN** the UI keeps that panel as geometry

#### Scenario: Null layout falls back to an empty leaf

- **WHEN** the UI requests a session whose `layout.json` is absent
- **THEN** the UI renders a single empty leaf and does not inherit the previous session's tree
