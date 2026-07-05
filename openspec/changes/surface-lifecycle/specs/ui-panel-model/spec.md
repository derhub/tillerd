## MODIFIED Requirements

### Requirement: Panel content type assignment

A panel leaf SHALL bind to at most one surface, identified by `placement`, and SHALL NOT own a
surface id. An empty leaf (no bound placement) SHALL present a picker; choosing a surface kind
SHALL spawn a surface -- the orchestrator appends a launch item to the session spec and mints the
placement -- and the acting leaf SHALL bind to the returned placement and create the surface at it.
Binding by placement supersedes assigning a content type to a leaf.

Closing a leaf SHALL be content-dependent. Closing a leaf bound to a surface SHALL terminate that
surface and unbind the leaf back to empty, keeping the leaf and its geometry in the tree. Closing an
empty leaf SHALL remove it from the tree and collapse its parent split. A close SHALL never reduce
the tree below one leaf; a close that would remove the last leaf SHALL instead leave a single empty
leaf. Both the unbind-to-empty and the remove-leaf outcomes SHALL persist the updated geometry and
keep the launch spec in agreement.

#### Scenario: Empty leaf picker spawns a surface

- **WHEN** the user picks a surface kind in an empty leaf
- **THEN** the orchestrator appends a launch item and mints a placement, and the leaf binds to that placement and creates the surface at it

#### Scenario: Binding persists

- **WHEN** a leaf is bound to a placement
- **THEN** the leaf's placement binding is persisted and survives a reload

#### Scenario: Closing a bound leaf unbinds it to empty

- **WHEN** the user closes a leaf bound to a surface
- **THEN** the surface is terminated and the leaf is unbound back to empty in place, and the updated geometry is persisted

#### Scenario: Closing an empty leaf removes it

- **WHEN** the user closes an empty leaf that is not the only leaf
- **THEN** the leaf is removed, its parent split collapses, and the updated geometry is persisted

#### Scenario: Close never empties the tree

- **WHEN** a close would remove the last leaf of the session
- **THEN** the tree retains a single empty leaf instead of becoming empty
