# launch-spec Specification

## Purpose
TBD - created by archiving change launch-system. Update Purpose after archive.
## Requirements
### Requirement: Versioned launch spec schema

The launch spec SHALL be a JSON blob with an integer version field and an ordered list of launch
items. The version field SHALL be a positive integer starting at 1. The launch item list MAY be
empty. The spec SHALL be stored alongside its version as two separate values so the migration
engine can inspect the version without parsing the payload.

#### Scenario: Well-formed spec with items is accepted

- **WHEN** a spec blob carries version 1 and a valid ordered list of launch items
- **THEN** it is accepted as a valid launch spec

#### Scenario: Empty item list is valid

- **WHEN** a spec blob carries version 1 and an empty item list
- **THEN** it is accepted as a valid launch spec

#### Scenario: Missing version field is rejected

- **WHEN** a blob is parsed that lacks a version field
- **THEN** parsing returns a typed error and no spec is produced

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

### Requirement: Lazy migration engine

The orchestrator SHALL migrate a stored spec blob to the current version on load: it reads the
stored version integer, applies each migration step in order as a pure function from vN JSON
string to vN+1 JSON string, writes the result back to the store, and returns the parsed spec.
The engine SHALL be extendable: adding a new version requires only adding one pure migration
function; no other code changes to the engine loop are required.

#### Scenario: Current-version blob passes through without write-back

- **WHEN** the stored spec version equals the current version
- **THEN** the spec is returned after parsing with no write-back to the store

#### Scenario: Older blob is migrated and written back

- **WHEN** the stored spec version is older than the current version
- **THEN** each migration step is applied in order, the result is written back to the store, and the parsed spec is returned

#### Scenario: Unknown future version is refused

- **WHEN** the stored spec version is greater than the version the binary supports
- **THEN** the migration engine returns a typed error and does not serve the spec

