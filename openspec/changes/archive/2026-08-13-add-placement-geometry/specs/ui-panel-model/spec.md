## ADDED Requirements

### Requirement: Split groups own child size geometry

Every split group SHALL carry a `sizes` array containing one finite, non-negative percentage for each child in child order. At least one size SHALL be positive, and the percentages SHALL total 100 after normalization. A newly created split SHALL assign equal sizes. Missing, non-finite, negative, wrong-length, or non-normalizable sizes in persisted geometry SHALL be rejected.

#### Scenario: New split starts equal

- **WHEN** the user splits a panel into two children
- **THEN** the new group stores `[50, 50]` as its child sizes

#### Scenario: Nested split sizes stay independent

- **WHEN** a divider moves inside a nested split group
- **THEN** only that group's normalized sizes change and every other group's sizes remain unchanged

#### Scenario: Invalid child sizes are rejected

- **WHEN** a stored split group has missing, non-finite, negative, wrong-length, or all-zero sizes
- **THEN** layout deserialization rejects the stored geometry
