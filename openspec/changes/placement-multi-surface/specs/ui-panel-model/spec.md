## MODIFIED Requirements

### Requirement: Panel content type assignment

A panel leaf SHALL bind to at most one surface, identified by `placement`, and SHALL NOT own a
surface id. An empty leaf (no bound placement) SHALL present a picker; choosing a surface kind
SHALL spawn a surface -- the orchestrator appends a launch item to the session spec and mints the
placement -- and the acting leaf SHALL bind to the returned placement and create the surface at it.
Binding by placement supersedes assigning a content type to a leaf.

#### Scenario: Empty leaf picker spawns a surface

- **WHEN** the user picks a surface kind in an empty leaf
- **THEN** the orchestrator appends a launch item and mints a placement, and the leaf binds to that placement and creates the surface at it

#### Scenario: Binding persists

- **WHEN** a leaf is bound to a placement
- **THEN** the leaf's placement binding is persisted and survives a reload

### Requirement: Panel tree state model

The application SHALL maintain a panel tree that carries geometry (splits, sizes, tabs) and a
`placement` binding per leaf; a leaf SHALL NOT carry a surface id. On load the tree SHALL be
restored from the stored geometry: a leaf bound to a placement renders the surface resolved by
`(session, placement)`, and an empty (unbound) leaf is kept as durable geometry. A session with no
stored layout SHALL initialize to a single empty leaf and SHALL NOT inherit the previously-open
session's tree. Every structural change (split, close, placement binding) SHALL persist the updated
geometry; spawn and close write both the geometry and the launch spec so they stay in agreement. A
spec-authoritative reconcile against the launch spec (add a leaf for a spec placement with no leaf,
drop a leaf whose placement is absent from the spec) is a deferred follow-up.

#### Scenario: Bound leaf resolves its surface on load

- **WHEN** the panel tree is restored for a session and a leaf is bound to a placement
- **THEN** that leaf renders the surface resolved by `(session, placement)`

#### Scenario: Fresh session initializes to an empty leaf

- **WHEN** the panel tree is initialized for a session with no stored layout
- **THEN** it is a single empty leaf and does not inherit the previous session's tree

#### Scenario: Empty leaf survives a reload

- **WHEN** stored geometry carries an empty leaf with no bound placement
- **THEN** that leaf is kept as geometry and is not dropped

#### Scenario: Split persists

- **WHEN** the user splits a panel and reloads
- **THEN** the split group is restored from the persisted geometry
