## MODIFIED Requirements

### Requirement: Query and create commands carry a stable wire shape

A query command SHALL return a `*View` DTO whose JSON key set is asserted against the generated
bindings contract (`@tillerd/client-bindings`, tauri-specta output). A create command SHALL mint the
entity id at the transport, execute the core command, and read the entity back by that id to return
its `*View`. Argument shapes SHALL be primitive wire types (strings, numbers, primitive ids), never
domain newtypes.

#### Scenario: A query response matches the generated bindings shape

- **WHEN** a new query command returns a `*View`
- **THEN** a shape test serializes the view and asserts the exact camelCase key set the generated
  bindings declare

#### Scenario: A create reads back by minted id

- **WHEN** a create command runs
- **THEN** the transport mints the id, executes the core create (which returns no data), and reads the entity back by that id to return its `*View`
