## ADDED Requirements

### Requirement: Every app command and query is a registered desktop command

Every orchestrator `app/` use-case handler (command or query) SHALL be reachable from the desktop client as a registered `#[tauri::command]`, listed in `collect_transport!`. A renderer SHALL be able to invoke any app-layer use case by its command name without a missing-command error. The launch-template domain SHALL be wired into `app/mod.rs` so its handlers compile and can be exposed.

#### Scenario: An app handler has a desktop command

- **WHEN** the orchestrator defines an app-layer command or query use case
- **THEN** a desktop tauri command shim dispatches to it through the bus, and the command is listed in `collect_transport!`

#### Scenario: The registration contract test covers every command

- **WHEN** the desktop command-contract test runs
- **THEN** it enumerates every command in `collect_transport!`, invokes each with a representative argument body, and asserts none returns a "command not found" or "invalid args" error

### Requirement: Query and create commands carry a stable wire shape

A query command SHALL return a `*View` DTO whose JSON key set is asserted against the `@tillerd/sdk` contract. A create command SHALL mint the entity id at the transport, execute the core command, and read the entity back by that id to return its `*View`. Argument shapes SHALL be primitive wire types (strings, numbers, primitive ids), never domain newtypes.

#### Scenario: A query response matches the SDK shape

- **WHEN** a new query command returns a `*View`
- **THEN** a shape test serializes the view and asserts the exact camelCase key set the SDK declares

#### Scenario: A create reads back by minted id

- **WHEN** a create command runs
- **THEN** the transport mints the id, executes the core create (which returns no data), and reads the entity back by that id to return its `*View`

### Requirement: Exposure is additive and needs no ACL change

Exposing a command SHALL NOT change any existing command's name, argument shape, or response JSON, and SHALL NOT require a per-command entry in `capabilities/default.json` (the renderer's `tauri://localhost` local origin skips per-command ACL).

#### Scenario: No capability manifest entry is added

- **WHEN** a new desktop command is registered
- **THEN** it is callable from the local renderer with no addition to `capabilities/default.json`
