## MODIFIED Requirements

### Requirement: Panel content type assignment

A panel leaf SHALL bind to at most one surface, identified by `placement`, and SHALL NOT own a
surface id. An empty leaf (no bound placement) SHALL present a picker; choosing a surface kind
SHALL spawn a surface -- the orchestrator appends a launch item to the session spec, mints the
placement, and creates the surface -- and the acting leaf SHALL bind to the returned placement.
Binding by placement supersedes assigning a content type to a leaf.

#### Scenario: Empty leaf picker spawns a surface

- **WHEN** the user picks a surface kind in an empty leaf
- **THEN** the orchestrator appends a launch item, mints a placement, and creates the surface, and the leaf binds to that placement

#### Scenario: Binding persists

- **WHEN** a leaf is bound to a placement
- **THEN** the leaf's placement binding is persisted and survives a reload

### Requirement: Panel tree state model

The application SHALL maintain a panel tree that carries geometry (splits, sizes, tabs) and a
`placement` binding per leaf; a leaf SHALL NOT carry a surface id. The tree SHALL be initialized
by reconciling stored geometry against the session's launch-spec placements: every spec
placement gets a leaf (its stored geometry, or a default), a leaf bound to a placement absent
from the spec is dropped, and an empty (unbound) leaf is kept as durable geometry. Stored
geometry is a best-effort hint; the launch spec is authoritative for which surfaces exist. Every
structural change (split, close, placement binding) SHALL persist the updated geometry.

#### Scenario: Tree reconciles against the spec on load

- **WHEN** the panel tree is initialized for a session whose spec has a placement with no stored leaf
- **THEN** a default leaf is created for that placement and the surface binds to it

#### Scenario: Bound orphan leaf is dropped

- **WHEN** stored geometry carries a leaf bound to a placement absent from the session spec
- **THEN** that leaf is dropped on load and is not rendered

#### Scenario: Empty leaf survives reconciliation

- **WHEN** stored geometry carries an empty leaf with no bound placement
- **THEN** that leaf is kept as geometry and is not dropped

#### Scenario: Split persists

- **WHEN** the user splits a panel and reloads
- **THEN** the split group is restored from the persisted geometry
