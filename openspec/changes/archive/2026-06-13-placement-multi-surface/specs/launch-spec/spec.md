## MODIFIED Requirements

### Requirement: Launch item fields

Each launch item in a spec SHALL carry:
- `target`: the kind of surface to create (terminal or agent)
- `placement`: a slot id that binds the item's surface to a panel. A placement is minted
  by the orchestrator when the item enters a session spec (template instantiation, or a
  later spawn) and SHALL be unique within that session, so a session holds N distinctly-placed
  surfaces. A launch template carries no placement; placement exists only on a per-session
  spec. `placement` is a minted slot id and supersedes the former fixed `center`/`side` set.
- `command`: either a reference to a named command in the library, or an inline command
  specification carrying the executable path, argument list, and environment overrides
- `pre`: an ordered list of shell strings to run before the command starts; MAY be empty
- `post`: an ordered list of shell strings to run after the command starts; MAY be empty
- `auto_spawn`: an ordered list of shell strings that run on each surface attach; MAY be empty
- `worktree`: an optional worktree step; absent means no worktree is created

#### Scenario: Item with library command reference is accepted

- **WHEN** a launch item names an existing command library entry as its command
- **THEN** the item is valid and the command reference is resolved at execution time

#### Scenario: Item with inline command is accepted

- **WHEN** a launch item specifies an inline executable path, argument list, and environment map
- **THEN** the item is valid and no library lookup is required

#### Scenario: Item with unknown command reference is rejected at execution

- **WHEN** a launch item names a command that does not exist in the library
- **THEN** the executor returns a typed error for that item at execution time, not at parse time

#### Scenario: Placement minted when an item enters a session spec

- **WHEN** a template is instantiated into a session spec, or a surface is spawned into a session
- **THEN** the orchestrator mints a placement unique within that session for the new item

#### Scenario: Placement is unique within a session spec

- **WHEN** a session spec is validated with two launch items carrying the same placement
- **THEN** the spec is rejected with a typed error naming the duplicate placement
