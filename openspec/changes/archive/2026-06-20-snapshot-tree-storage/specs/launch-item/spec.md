## MODIFIED Requirements

### Requirement: Best-effort failure model

A failed launch item SHALL NOT prevent remaining items from executing. When an item fails
(pre-script error, command-not-found, surface creation error) the executor
SHALL record a typed error status on the surface row for that item (or a placeholder row if
surface creation itself failed) and SHALL proceed to the next item in the list.

#### Scenario: Failed item does not block subsequent items

- **WHEN** an item fails at any stage (pre-script, command resolution, or surface creation)
- **THEN** the executor records the error for that item and continues executing the remaining items

#### Scenario: Error is observable on the surface row

- **WHEN** a launch item fails
- **THEN** the surface row (or placeholder) carries a typed error status that can be retrieved
