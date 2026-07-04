# ui-command-manager

## MODIFIED Requirements

### Requirement: Surface tags project commands to UI locations

Each command definition SHALL declare the surfaces it appears in (palette, title bar,
context menu, activity bar, status bar), defaulting to the palette. A UI surface SHALL
render exactly the commands tagged for it whose `when` currently passes, ordered by
group. Chrome toolbars (title bar, activity bar, status bar) and context menus SHALL be
projections of the commands tagged for them, requiring no hand-wired button or item
lists.

#### Scenario: Command appears only on its tagged surfaces

- **WHEN** a command is tagged for the title bar only
- **THEN** it renders as a title bar control and does not appear in the palette

#### Scenario: Title bar toolbar is data-driven

- **WHEN** a command tagged for the title bar is added to the definitions
- **THEN** its control appears in the title bar toolbar without editing the toolbar
  component

#### Scenario: Context menus are data-driven

- **WHEN** a command tagged for the context menu with a row-scope `when` is added to the
  definitions
- **THEN** it appears in the matching rows' context menus without editing the menu
  component

## ADDED Requirements

### Requirement: Command invocation arguments

A command handler SHALL be invocable with an optional argument payload (e.g. the entity
id and kind of the row a context menu was opened on). Surfaces that carry target context
SHALL pass it on invocation; handlers registered without argument support keep working
unchanged.

#### Scenario: Context menu passes the row entity

- **WHEN** the user invokes "Archive" from a session row's context menu
- **THEN** the archive handler receives that session's id without reading global state

#### Scenario: Palette invocation without arguments still works

- **WHEN** a no-argument command is invoked from the palette
- **THEN** its handler runs exactly as before
