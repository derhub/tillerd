## MODIFIED Requirements

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
