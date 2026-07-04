# ui-template-manager

## ADDED Requirements

### Requirement: Templates sidebar view

The Templates activity-bar view SHALL present the portable template library (prebuilt and
custom) and, when a project context is active, that project's launch templates. Rows SHALL
show the template name and origin; pinned templates list first.

#### Scenario: Library and project sections

- **WHEN** the Templates view opens while a project is active
- **THEN** the portable library and the active project's launch templates render as
  distinct sections

### Requirement: Visual launch-spec editor

Creating or editing a template (or a project launch template) SHALL open a visual form
editor over the launch spec: an ordered list of launch items where each item selects a
command from the command library, a placement, an optional working directory, and
environment rows. Items SHALL be addable, removable, and reorderable. Saving SHALL
serialize to the versioned spec and apply it through the existing apply operations. Raw
spec JSON SHALL NOT be required in this flow.

#### Scenario: Adding a launch item

- **WHEN** the user adds an item, picks a command from the library, and saves
- **THEN** the template's spec contains the new item and a session created from the
  template spawns that surface

#### Scenario: Reordering launch items

- **WHEN** the user reorders items and saves
- **THEN** the persisted spec lists the items in the new order

#### Scenario: Invalid item is rejected inline

- **WHEN** the user saves an item without a command selected
- **THEN** the editor surfaces an inline validation error and does not apply the spec

### Requirement: Template import and export

The view SHALL support importing a template from a file and exporting a template to a
file, surfacing success or failure through the notification center.

#### Scenario: Export then import round-trips

- **WHEN** the user exports a template and imports the produced file
- **THEN** a template with the same name and spec appears in the library

### Requirement: Template row actions

Template rows SHALL expose pin/unpin, delete (confirmed; prebuilt library templates
excluded), and export through context menu and hover actions wired through the command
registry. Project launch-template rows SHALL expose edit and discard.

#### Scenario: Prebuilt template delete is rejected

- **WHEN** the user opens a prebuilt library template's context menu
- **THEN** delete is absent or disabled while pin and export are offered
