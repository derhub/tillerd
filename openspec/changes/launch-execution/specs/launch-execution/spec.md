# Launch Execution

## ADDED Requirements

### Requirement: Launch items execute in declared order, best-effort

The executor SHALL run a session's launch items in their declared order. A failed item SHALL NOT
abort the run; its failure SHALL be recorded as that item's outcome and the remaining items SHALL
still execute.

#### Scenario: All items run in order
- **WHEN** a launch template with three valid items is instantiated into a session
- **THEN** three surfaces are created, in the items' declared order

#### Scenario: A failed item does not abort the run
- **WHEN** the second of three items fails
- **THEN** the second item's outcome records the error, no surface is created for it, and the first and third items still produce their surfaces

### Requirement: An item's command is resolved before launch

The executor SHALL resolve each item's command before launching: a command-library reference SHALL
resolve to the stored executable, arguments, and environment; an inline command SHALL be used as
given. An unresolvable reference SHALL fail the item with a typed not-found error and create no
surface.

#### Scenario: Library reference resolves
- **WHEN** an item references a command-library entry by name
- **THEN** the stored executable, arguments, and environment are used to launch the surface

#### Scenario: Inline command is used as given
- **WHEN** an item carries an inline command
- **THEN** that executable, arguments, and environment are used

#### Scenario: Unknown reference fails the item
- **WHEN** an item references a command name absent from the library
- **THEN** the item outcome records a not-found error and no surface is created

### Requirement: An item is dispatched by its target kind

The executor SHALL select a surface adapter by the item's target kind and delegate the surface's
creation to it. A target with no registered adapter SHALL fail the item with a typed
unsupported-kind error.

#### Scenario: Target selects the adapter
- **WHEN** one item targets a terminal and another targets an agent
- **THEN** each surface is created by the adapter for its kind

#### Scenario: Unsupported target fails loudly
- **WHEN** an item targets a kind with no registered adapter
- **THEN** the item outcome records an unsupported-kind error and no surface is created

### Requirement: A worktree step runs before the surface and sets its working directory

WHEN an item declares a worktree step, the executor SHALL run it before creating the surface, use
the resulting path as the surface's working directory, and record the worktree on the surface. The
step SHALL run against an explicit repository root, not the process working directory. A failing
worktree step SHALL fail the item and create no surface.

#### Scenario: Worktree step provides the working directory
- **WHEN** an item declares a worktree step
- **THEN** the step runs against the project's repository root, the surface's working directory is the worktree path, and the surface records the worktree identifier

#### Scenario: Worktree step failure fails the item
- **WHEN** the worktree step returns an error
- **THEN** the item outcome records the error and no surface is created

### Requirement: Item placement is recorded on the surface

The executor SHALL store an item's placement on the created surface so the host can position it.

#### Scenario: Placement is persisted
- **WHEN** an item declares a placement region
- **THEN** the created surface records that placement

### Requirement: A session instantiated from a template copies its launch spec atomically

WHEN a session is created from a launch template, the session's launch spec and its version SHALL be
copied from the template in a single atomic operation. Updating the launch spec of an absent
template SHALL return a typed not-found error.

#### Scenario: Template spec is copied on session creation
- **WHEN** a session is created referencing a launch template
- **THEN** the session carries a copy of the template's launch spec and version, and later diverges independently

#### Scenario: Updating an absent template is not-found
- **WHEN** the launch spec of a template identifier that does not exist is updated
- **THEN** a typed not-found error is returned and nothing is written
