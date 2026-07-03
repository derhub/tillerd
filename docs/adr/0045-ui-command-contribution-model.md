# 0045. UI command contribution model

- Status: accepted
- Date: 2026-07-03

## Context

Renderer UI commands were declared across three disconnected sites: ids and titles in `commands/ids.ts`, default keybindings in `commands/keybindings.ts`, and `run` handlers scattered across shell components. Adding a command meant editing three files, there was no way to gate a command by context, no model for where a command surfaces (palette, title bar, context menu), no icon or toggle state, and the command palette listed every command unconditionally even though the `command-center` spec promised actions "available in context". A custom desktop title bar needs a data-driven toolbar of toggle commands, which the ad-hoc split cannot provide.

## Decision

Renderer UI commands SHALL follow a single contribution model:

- Each command is declared once as a static `CommandDef` (id, title, category, keywords, icon, surface tags, group, per-preset default keys, an optional `when` context expression, and an optional toggle selector). The set of definitions is the single source of truth for command identity, titles, and default keybindings.
- Handlers are registered by id at runtime, decoupled from the definition, so a handler may close over live application context. The runtime command is the composition of definition + handler + resolved accelerator + resolved checked state.
- A reactive context-key store plus a minimal `when` evaluator (conjunction of context-key terms with negation) gates palette visibility, keybinding activation, and command enablement. Absent `when` means always available.
- Surface tags (`palette`, `titlebar`, `contextmenu`) project commands to UI locations; each surface renders exactly the commands tagged for it whose `when` passes, ordered by group. Toolbars are projections of the table, not hand-wired button lists.
- Toggle commands compute checked state from a selector against current state; checked state is never stored separately from the state it reflects.
- The existing canonical accelerator format, presets, per-action overrides, leader key, mac-aware display, and settings keys are preserved; only the default-key source moves into the definitions. Keybindings remain single-chord.

## Consequences

- One declaration site per command; the palette, keybinding resolution, global shortcut dispatch, and every toolbar/menu derive from it.
- Context gating becomes real, satisfying the `command-center` "available in context" requirement, and keybindings no longer fire out of context.
- The desktop title bar toolbar (and future context menus) are data-driven off command definitions.
- A broad one-time migration of every command registration site, done incrementally behind the stable `useCommands()` consumer contract; user keybinding config is untouched.
- Scope is deliberately minimal: no multi-key sequences, no full when-clause grammar, no full menus-contribution map. The `when` and surface types are forward-compatible so a richer expression or menu model can replace them later without changing call sites. A superseding ADR is required to change this decision.
