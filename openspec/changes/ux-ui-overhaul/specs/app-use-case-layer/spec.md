# app-use-case-layer

## ADDED Requirements

### Requirement: Placement swap

The app layer SHALL expose a `SwapPlacement` command taking a session id and two
placements, atomically swapping the placement bindings of the two surfaces in one
transaction. The command SHALL fail without change when either placement resolves to no
surface in that session. The operation is additive: wire protocol, ACL model, and
existing operations are unchanged.

#### Scenario: Swap succeeds atomically

- **WHEN** SwapPlacement runs for two placements each bound to a surface in the session
- **THEN** after commit each surface carries the other's placement and both PTYs keep
  running

#### Scenario: Unknown placement fails without change

- **WHEN** SwapPlacement runs with a placement not bound to any surface in the session
- **THEN** the command errors and neither surface's placement changes
