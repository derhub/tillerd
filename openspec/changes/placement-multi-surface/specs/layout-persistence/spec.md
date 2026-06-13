## MODIFIED Requirements

### Requirement: Layout restored on session open

The orchestrator SHALL return the stored `layout_json` blob when a get-session or open-session request is made for a session that has a persisted layout. On open, the UI SHALL reconcile the stored geometry against the session's launch-spec placements before rendering: every spec placement SHALL get a panel (its stored geometry, or a default leaf appended to the root group); a leaf bound to a placement absent from the spec SHALL be dropped; an empty (unbound) leaf SHALL be kept as durable geometry. The launch spec is authoritative for which surfaces exist; stored geometry is a best-effort hint that self-heals. When no layout has been stored (`layout_json` is NULL) the UI SHALL render a default layout derived from the spec's placements, falling back to a single empty leaf when the spec has none.

#### Scenario: Stored layout returned on open

- **WHEN** the UI requests a session that has a persisted layout
- **THEN** the response includes the `layout_json` blob exactly as stored

#### Scenario: Reconciliation adds a panel for a new spec placement

- **WHEN** the stored geometry has no panel for a placement present in the session spec
- **THEN** the UI creates a default panel for that placement and binds its surface

#### Scenario: Reconciliation drops a bound orphan panel

- **WHEN** the stored geometry has a panel bound to a placement absent from the session spec
- **THEN** the UI drops that panel and does not render it

#### Scenario: Reconciliation keeps an empty panel

- **WHEN** the stored geometry has an empty panel with no bound placement
- **THEN** the UI keeps that panel as geometry

#### Scenario: Null layout for an empty spec falls back to an empty leaf

- **WHEN** the UI requests a session whose `layout_json` is NULL and whose spec has no placements
- **THEN** the UI renders a single empty leaf
