# ui-command-manager Specification

## Purpose

The command manager is the single contribution model for renderer UI commands: each command is declared once (identity, presentation, surfaces, default keybindings, availability, toggle state), handlers register by id, and one resolution path feeds the command palette, global keyboard shortcuts, the keybinding settings, and any toolbar or menu. It replaces the former split between id/title constants, a separate keybinding preset map, and scattered handler registration.
## Requirements
### Requirement: Single command declaration

Every UI command SHALL be declared once as a command definition carrying a stable id, a title, and optional metadata: category, keywords, icon, surface tags, group, default keybindings per preset, a context (`when`) expression, and a toggle selector. The set of definitions SHALL be the single source of truth for command identity, titles, and default keybindings. No command's identity, title, or default key SHALL be declared outside this set.

#### Scenario: A command is fully described by its definition

- **WHEN** a command definition declares id, title, default keys, surfaces, and a when expression
- **THEN** the palette, keybinding resolution, and surface projection all read that single definition without a second declaration site

#### Scenario: Adding a command touches one declaration

- **WHEN** a new command is introduced with its metadata and default key
- **THEN** it appears in the palette and resolves its default binding without editing a separate id list or keybinding map

### Requirement: Handlers register by id, separate from declaration

A command's behavior SHALL be registered by id at runtime, decoupled from its static definition, so a handler MAY close over live application context (navigation, stores, session). A command SHALL be active only while a handler is registered for its id: an inactive command SHALL NOT be surfaced in any UI location and its keybinding SHALL NOT fire (the keystroke SHALL pass through untouched). This keeps availability tied to a live implementation, so a not-yet-mounted or host-inapplicable contributor never shows a dead entry or swallows a shortcut.

#### Scenario: Handler closes over live context

- **WHEN** a handler registered for a command id calls navigation or a store setter
- **THEN** invoking that command from the palette or its keybinding runs the handler against current context

#### Scenario: Command with no registered handler is inactive

- **WHEN** a command is defined but no handler is registered for its id
- **THEN** it does not appear in the palette or any surface, and pressing its default key does not prevent the keystroke's default handling

#### Scenario: A contributor mounting activates its commands

- **WHEN** a component that registers a command's handler mounts
- **THEN** the command becomes visible on its surfaces and its keybinding begins firing; unmounting reverses this

### Requirement: Context-key store and `when` evaluation

The system SHALL maintain a reactive store of named context keys and SHALL evaluate a command's `when` expression against it. A `when` expression SHALL support a conjunction of context-key terms with negation (a term is a key that must be truthy, or a negated key that must be falsy). A command with no `when` SHALL always be available. When a context key changes, dependent command availability SHALL update reactively.

#### Scenario: Command available only when context holds

- **WHEN** a command's `when` requires `hasActiveSession` and no session is active
- **THEN** the command is unavailable (hidden in the palette and its keybinding does not fire)

#### Scenario: Negated term excludes context

- **WHEN** a command's `when` is `!terminalFocus` and a terminal surface holds focus
- **THEN** the command is unavailable

#### Scenario: Availability updates reactively

- **WHEN** a context key flips from false to true while the palette is open
- **THEN** a command gated on that key becomes visible without reopening the palette

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

### Requirement: First-class toggle commands

A command definition MAY declare a toggle selector, making it a toggle command. Its checked state SHALL be computed from the selector against current context and SHALL NOT be stored separately from the underlying state it reflects. Surfaces rendering a toggle command SHALL show its checked/unchecked state; invoking it SHALL run its handler (which mutates the underlying state) and the checked state SHALL follow that mutation.

#### Scenario: Checked state reflects the source

- **WHEN** the state a toggle command reflects is on
- **THEN** the command renders checked in the palette and pressed in the title bar toolbar

#### Scenario: Invoking a toggle flips the source and the display

- **WHEN** the user invokes a toggle command that is currently checked
- **THEN** its handler turns the underlying state off and the command renders unchecked everywhere it appears

### Requirement: Single keybinding resolution over the definitions

Default keybindings SHALL be sourced from the command definitions' per-preset default keys. The existing canonical accelerator format, preset selection, per-action overrides, leader key, and mac-aware display SHALL be preserved and SHALL resolve against the definitions. A resolved keybinding SHALL fire its command only when the command's `when` currently passes. Keybindings SHALL remain single-chord.

#### Scenario: Default key comes from the definition

- **WHEN** a command declares a default key for the active preset and has no override
- **THEN** its resolved binding is that default key

#### Scenario: Override still wins and persists

- **WHEN** a user overrides a command's key
- **THEN** the resolved binding is the override, it persists across restarts, and clearing it falls back to the definition's preset default

#### Scenario: Gated keybinding does not fire out of context

- **WHEN** a command's `when` is false and the user presses its resolved key outside a capture target
- **THEN** the command does not run

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

