## MODIFIED Requirements

### Requirement: Placement hint on surface creation

A launch item in a session spec SHALL carry a placement slot id, minted by the orchestrator
and unique within the session, stored on the surface row at creation time. The placement is
the durable key the UI uses to bind the surface to a panel in the panel tree. A launch
template carries no placement; the orchestrator mints one when the item enters a session spec
(instantiation or spawn). Placements are never reused: a fresh placement is minted per spawn,
and a closed placement is retired. Two launch items in a session spec SHALL NOT share a
placement.

#### Scenario: Placement stored on the surface row

- **WHEN** a launch item produces a surface
- **THEN** the surface row records the item's minted placement slot id

#### Scenario: Placement minted at spawn

- **WHEN** a surface is spawned into a session
- **THEN** the orchestrator mints a placement unique within that session for the new item

#### Scenario: Duplicate placement is rejected

- **WHEN** a spec would create two surfaces at the same placement
- **THEN** creation returns a typed error and the second surface is not created
