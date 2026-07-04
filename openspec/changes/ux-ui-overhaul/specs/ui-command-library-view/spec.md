# ui-command-library-view

## ADDED Requirements

### Requirement: Command library sidebar view

The Commands activity-bar view SHALL list the command library (prebuilt and custom
commands), pinned commands first, each row showing the command name and its CLI. The list
SHALL paginate or virtualize rather than assume a small fixed set.

#### Scenario: Library lists both origins

- **WHEN** the Commands view opens with prebuilt and custom commands present
- **THEN** both are listed, with pinned commands grouped first and each origin
  distinguishable

### Requirement: Command create and edit

The view SHALL provide a create affordance opening a form with name, CLI, arguments, and
environment rows (key/value add/remove). Editing a custom command SHALL reuse the same
form pre-filled. Saving SHALL validate that name and CLI are non-empty and surface
rejection errors inline.

#### Scenario: Creating a custom command

- **WHEN** the user submits the create form with a name and CLI
- **THEN** the new command appears in the list without a manual refresh

#### Scenario: Editing a custom command

- **WHEN** the user edits a custom command's arguments and saves
- **THEN** the updated arguments persist and render in the row

### Requirement: Prebuilt commands are read-only

Prebuilt commands SHALL NOT offer rename, edit, or delete. Duplicating a prebuilt command
SHALL create an editable custom copy.

#### Scenario: Prebuilt row offers no destructive actions

- **WHEN** the user opens a prebuilt command's context menu
- **THEN** rename, edit, and delete are absent or disabled while duplicate and pin are
  offered

#### Scenario: Duplicate yields an editable copy

- **WHEN** the user duplicates a prebuilt command
- **THEN** a custom command with the same CLI appears and is editable

### Requirement: Command row actions

Each command row SHALL expose rename (custom only), duplicate, pin/unpin, and delete
(custom only, confirmed) through its context menu and hover actions, wired through the
command registry.

#### Scenario: Deleting a custom command

- **WHEN** the user deletes a custom command and confirms
- **THEN** the command disappears from the list and from command pickers
