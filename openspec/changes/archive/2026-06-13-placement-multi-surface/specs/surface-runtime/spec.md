## ADDED Requirements

### Requirement: Resume a surface by session and placement

The runtime SHALL resolve a session's surface by the pair `(session, placement)` for any surface
kind and any count, generalizing the former terminal-only, one-per-session resume. On revisit, the
UI SHALL re-attach each of a session's panels to the surface at its placement by resolving
`(session, placement)`. The lookup SHALL return at most one live surface, or report absence; a
placement with no live surface (never spawned, or closed) is a normal absence, not an error. A
closed surface SHALL NOT be resumed.

#### Scenario: Each panel re-attaches its placement's surface on revisit

- **WHEN** a session with surfaces at two distinct placements is revisited
- **THEN** each panel re-attaches the surface resolved by its `(session, placement)` pair, and neither panel shows the other placement's surface

#### Scenario: Absence at a placement is reported, not substituted

- **WHEN** `(session, placement)` resolves to no live surface
- **THEN** the runtime returns a normal absence result and does not attach a surface from a different placement

## MODIFIED Requirements

### Requirement: Placement hint accepted at surface creation

Surface creation SHALL record a placement slot id on the surface row, minted by the orchestrator
and unique within the session. The placement is the durable key by which a surface is resolved
for a session: a `(session, placement)` pair SHALL identify at most one live surface. The
placement has no effect on the proxy or pseudo-terminal assignment; it is the binding key between
a surface and the panel that renders it. Creating a second live surface at a placement already in
use within a session SHALL be rejected with a typed error.

#### Scenario: Placement recorded when surface is created

- **WHEN** a surface is created for a session
- **THEN** the surface row records its minted placement and the surface is resolvable by `(session, placement)`

#### Scenario: Duplicate placement within a session is rejected

- **WHEN** a surface creation call targets a placement already held by a live surface in the same session
- **THEN** the runtime returns a typed conflict error and no second surface is created
