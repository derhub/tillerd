## ADDED Requirements

### Requirement: Ordered item execution

The executor SHALL process launch items from a spec in the order they appear in the item list.
Each item SHALL be executed sequentially: a new item SHALL NOT start until the previous item
has completed its pre-scripts, surface creation, and post-scripts. The spec's item order is
the authoritative execution order.

#### Scenario: Items run in list order

- **WHEN** a spec contains items A, B, and C in that order
- **THEN** A's surface is created and its post-scripts run before B starts, and B's before C starts

### Requirement: Pre and post script execution

For each launch item the executor SHALL run the item's pre-script list, in order, before
initiating surface creation, and SHALL run the post-script list, in order, after the surface
process has started. Each script entry is a shell string executed in the item's resolved
working directory. A script that exits with a non-zero code SHALL be treated as a fatal error
for that item: the surface is not created, the error is recorded on the surface row, and
execution advances to the next item.

#### Scenario: Pre-scripts run before surface creation

- **WHEN** an item has pre-scripts
- **THEN** all pre-scripts complete before the surface process is started

#### Scenario: Post-scripts run after surface starts

- **WHEN** an item has post-scripts
- **THEN** all post-scripts run after the surface process has started

#### Scenario: Failing pre-script skips the surface

- **WHEN** a pre-script exits non-zero
- **THEN** the surface is not created, a typed error is recorded on the surface row, and the executor continues with the next item

### Requirement: Auto-spawn scripts

An item MAY carry an auto-spawn list. Each string in the list SHALL be injected into the
surface's input stream as a shell command on every surface attach — both the initial attach
at creation time and every subsequent reattach. Auto-spawn strings SHALL be delivered in list
order. Auto-spawn applies only to terminal surfaces; the behavior on agent surfaces is
undefined and implementations MAY ignore it.

#### Scenario: Auto-spawn injected on initial attach

- **WHEN** a terminal surface has auto-spawn strings and it is attached for the first time
- **THEN** each string is injected into the surface's input stream in order

#### Scenario: Auto-spawn injected on reattach

- **WHEN** a terminal surface with auto-spawn strings is reattached after a detach
- **THEN** the auto-spawn strings are injected again in order

### Requirement: Best-effort failure model

A failed launch item SHALL NOT prevent remaining items from executing. When an item fails
(pre-script error, command-not-found, surface creation error, worktree error) the executor
SHALL record a typed error status on the surface row for that item (or a placeholder row if
surface creation itself failed) and SHALL proceed to the next item in the list.

#### Scenario: Failed item does not block subsequent items

- **WHEN** an item fails at any stage (pre-script, command resolution, surface creation, or worktree step)
- **THEN** the executor records the error for that item and continues executing the remaining items

#### Scenario: Error is observable on the surface row

- **WHEN** a launch item fails
- **THEN** the surface row (or placeholder) carries a typed error status that can be retrieved

### Requirement: Placement hint on surface creation

Each launch item MAY carry a placement string. When present, the placement string SHALL be
stored on the surface row at creation time. The UI uses this stored value to route the surface
to the named region of the panel tree. When absent, the surface row's placement field SHALL
be null and the UI falls back to the default content region.

#### Scenario: Placement stored when present

- **WHEN** a launch item carries a non-empty placement string
- **THEN** the surface row records that placement value

#### Scenario: Null placement falls back to default

- **WHEN** a launch item has no placement
- **THEN** the surface row's placement field is null and the UI places the surface in the default content region
